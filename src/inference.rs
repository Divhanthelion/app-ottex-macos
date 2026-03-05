use anyhow::{Context, Result};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, model::Whisper, Config};
use std::sync::mpsc;
use tokenizers::Tokenizer;

use crate::config::LocalModelConfig;

/// Commands sent to the inference supervisor thread
pub enum InferenceCommand {
    /// Load the model with the given configuration
    LoadModel {
        config: LocalModelConfig,
        response: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// Transcribe PCM audio (f32, 16kHz, mono)
    Transcribe {
        audio_f32_16khz: Vec<f32>,
        response: tokio::sync::oneshot::Sender<Result<String>>,
    },
    /// Shut down the supervisor thread
    Shutdown,
}

/// Handle to send commands to the inference supervisor
pub struct InferenceSupervisor {
    tx: mpsc::Sender<InferenceCommand>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl InferenceSupervisor {
    /// Spawn the inference supervisor on a dedicated OS thread.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<InferenceCommand>();

        let thread = std::thread::Builder::new()
            .name("inference-supervisor".into())
            .spawn(move || {
                Self::run_loop(rx);
            })
            .expect("Failed to spawn inference supervisor thread");

        Self {
            tx,
            thread: Some(thread),
        }
    }

    /// Send a command to load the model. Returns when loading is complete.
    pub async fn load_model(&self, config: LocalModelConfig) -> Result<()> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(InferenceCommand::LoadModel {
                config,
                response: resp_tx,
            })
            .map_err(|_| anyhow::anyhow!("Inference supervisor channel closed"))?;
        resp_rx
            .await
            .map_err(|_| anyhow::anyhow!("Inference supervisor dropped response"))?
    }

    /// Send audio for transcription. Returns the transcribed text.
    pub async fn transcribe(&self, audio_f32_16khz: Vec<f32>) -> Result<String> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(InferenceCommand::Transcribe {
                audio_f32_16khz,
                response: resp_tx,
            })
            .map_err(|_| anyhow::anyhow!("Inference supervisor channel closed"))?;
        resp_rx
            .await
            .map_err(|_| anyhow::anyhow!("Inference supervisor dropped response"))?
    }

    /// Check if the supervisor thread is still alive
    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        self.thread
            .as_ref()
            .map(|t| !t.is_finished())
            .unwrap_or(false)
    }

    fn run_loop(rx: mpsc::Receiver<InferenceCommand>) {
        let mut engine: Option<WhisperEngine> = None;

        loop {
            let cmd = match rx.recv() {
                Ok(cmd) => cmd,
                Err(_) => {
                    log::info!("Inference supervisor channel closed, shutting down");
                    break;
                }
            };

            match cmd {
                InferenceCommand::LoadModel { config, response } => {
                    log::info!("Loading Whisper model: {}", config.repo_id);
                    let result = WhisperEngine::load(&config);
                    match &result {
                        Ok(_) => log::info!("Whisper model loaded successfully"),
                        Err(e) => log::error!("Failed to load Whisper model: {}", e),
                    }
                    let is_ok = result.is_ok();
                    if is_ok {
                        engine = result.ok();
                    }
                    let _ = response.send(if is_ok {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("Failed to load model"))
                    });
                }
                InferenceCommand::Transcribe {
                    audio_f32_16khz,
                    response,
                } => {
                    let result = match &mut engine {
                        Some(eng) => eng.transcribe(&audio_f32_16khz),
                        None => Err(anyhow::anyhow!(
                            "Model not loaded — cannot transcribe"
                        )),
                    };
                    let _ = response.send(result);
                }
                InferenceCommand::Shutdown => {
                    log::info!("Inference supervisor shutting down");
                    break;
                }
            }
        }
    }
}

impl Drop for InferenceSupervisor {
    fn drop(&mut self) {
        let _ = self.tx.send(InferenceCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Whisper inference engine using candle-transformers
struct WhisperEngine {
    model: Whisper,
    tokenizer: Tokenizer,
    mel_filters: Vec<f32>,
    config: Config,
    device: Device,
    // Special token IDs
    sot_token: u32,
    eot_token: u32,
    transcribe_token: u32,
    no_timestamps_token: u32,
    suppress_tokens: Tensor,
}

impl WhisperEngine {
    fn load(local_config: &LocalModelConfig) -> Result<Self> {
        let device = Device::new_metal(0).unwrap_or(Device::Cpu);

        // Use hf-hub to download/cache model files
        let api = hf_hub::api::sync::Api::new().context("Failed to create HF Hub API")?;
        let repo = api.repo(hf_hub::Repo::with_revision(
            local_config.repo_id.clone(),
            hf_hub::RepoType::Model,
            local_config.revision.clone(),
        ));

        log::info!("Downloading/caching model files from {}...", local_config.repo_id);

        let config_path = repo
            .get("config.json")
            .context("Failed to download config.json")?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .context("Failed to download tokenizer.json")?;
        let weights_path = repo
            .get("model.safetensors")
            .context("Failed to download model.safetensors")?;

        log::info!("Model files cached, loading...");

        // Load config
        let config_str =
            std::fs::read_to_string(&config_path).context("Failed to read config.json")?;
        let config: Config =
            serde_json::from_str(&config_str).context("Failed to parse config.json")?;

        // Load mel filters from embedded bytes based on num_mel_bins
        let mel_filters = Self::load_mel_filters(config.num_mel_bins)?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, &device)
                .context("Failed to load model weights")?
        };
        let model = Whisper::load(&vb, config.clone()).context("Failed to build Whisper model")?;

        // Resolve special tokens
        let sot_token = Self::token_id(&tokenizer, m::SOT_TOKEN)?;
        let eot_token = Self::token_id(&tokenizer, m::EOT_TOKEN)?;
        let transcribe_token = Self::token_id(&tokenizer, m::TRANSCRIBE_TOKEN)?;
        let no_timestamps_token = Self::token_id(&tokenizer, m::NO_TIMESTAMPS_TOKEN)?;

        // Build suppress tokens tensor
        let suppress_tokens = Self::build_suppress_tokens(&config, &device)?;

        log::info!(
            "Whisper model ready (encoder: {} layers, decoder: {} layers, mel bins: {})",
            config.encoder_layers,
            config.decoder_layers,
            config.num_mel_bins
        );

        Ok(Self {
            model,
            tokenizer,
            mel_filters,
            config,
            device,
            sot_token,
            eot_token,
            transcribe_token,
            no_timestamps_token,
            suppress_tokens,
        })
    }

    fn load_mel_filters(num_mel_bins: usize) -> Result<Vec<f32>> {
        // The mel filter bytes are standard Whisper assets.
        // We embed the 80-bin and 128-bin versions.
        let mel_bytes: &[u8] = match num_mel_bins {
            80 => include_bytes!("mel_filters_80.bytes"),
            128 => include_bytes!("mel_filters_128.bytes"),
            n => anyhow::bail!("Unsupported num_mel_bins: {}", n),
        };

        let mut filters = vec![0f32; mel_bytes.len() / 4];
        use byteorder::{ByteOrder, LittleEndian};
        LittleEndian::read_f32_into(mel_bytes, &mut filters);
        Ok(filters)
    }

    fn token_id(tokenizer: &Tokenizer, token: &str) -> Result<u32> {
        tokenizer
            .token_to_id(token)
            .ok_or_else(|| anyhow::anyhow!("Token not found in tokenizer: {}", token))
    }

    fn build_suppress_tokens(config: &Config, device: &Device) -> Result<Tensor> {
        let mut suppress = vec![0f32; config.vocab_size];
        for &token_id in &config.suppress_tokens {
            if (token_id as usize) < config.vocab_size {
                suppress[token_id as usize] = f32::NEG_INFINITY;
            }
        }
        let tensor =
            Tensor::from_vec(suppress, config.vocab_size, device).context("suppress tensor")?;
        Ok(tensor)
    }

    /// Transcribe f32 PCM audio at 16kHz mono.
    fn transcribe(&mut self, audio_f32: &[f32]) -> Result<String> {
        let start = std::time::Instant::now();

        // Pad or truncate to 30-second chunks and process each
        let mut all_text = String::new();
        let chunk_samples = m::N_SAMPLES; // 480000 = 30s at 16kHz

        let mut offset = 0;
        while offset < audio_f32.len() {
            let end = (offset + chunk_samples).min(audio_f32.len());
            let chunk = &audio_f32[offset..end];

            // Pad to exactly 30 seconds if shorter
            let padded: Vec<f32> = if chunk.len() < chunk_samples {
                let mut p = chunk.to_vec();
                p.resize(chunk_samples, 0.0);
                p
            } else {
                chunk.to_vec()
            };

            let text = self.transcribe_chunk(&padded)?;
            all_text.push_str(&text);

            offset += chunk_samples;
        }

        let elapsed = start.elapsed();
        log::info!(
            "Transcription complete in {:.2}s for {:.2}s of audio",
            elapsed.as_secs_f32(),
            audio_f32.len() as f32 / 16000.0
        );

        Ok(all_text.trim().to_string())
    }

    fn transcribe_chunk(&mut self, audio_30s: &[f32]) -> Result<String> {
        // Reset KV cache for fresh decoding
        self.model.reset_kv_cache();
        // Compute mel spectrogram using candle's built-in function
        let mel = m::audio::pcm_to_mel(&self.config, audio_30s, &self.mel_filters);
        let mel_len = mel.len();
        let n_mels = self.config.num_mel_bins;
        log::info!("audio_30s len: {}, mel_len: {}, n_mels: {}", audio_30s.len(), mel_len, n_mels);

        let frames = mel_len / n_mels;
        let mut mel = Tensor::from_vec(mel, (1, n_mels, frames), &self.device)
            .context("Failed to create mel tensor")?;

        if frames > m::N_FRAMES {
            mel = mel.narrow(2, 0, m::N_FRAMES)
                .context("Failed to narrow mel tensor")?
                .contiguous()
                .context("Failed to make mel contiguous")?;
        }

        // Encode audio, then drop mel tensor to free GPU memory
        let audio_features = self
            .model
            .encoder
            .forward(&mel, true)
            .context("Encoder forward failed")?;
        drop(mel);

        // Build initial token sequence
        let mut tokens: Vec<u32> = vec![
            self.sot_token,
            // Language token for English (token ID for "<|en|>" is typically sot_token + 1 for en)
            self.sot_token + 1,
            self.transcribe_token,
            self.no_timestamps_token,
        ];

        let sample_len = 224; // Max tokens to generate

        // Wrap the decoding loop in an autorelease pool so Metal GPU
        // buffers created each iteration are freed promptly instead of
        // accumulating until the outer pool drains.
        // See: https://github.com/huggingface/candle/issues/2271
        for i in 0..sample_len {
            let next_token = objc2::rc::autoreleasepool(|_| -> Result<u32> {
                let tokens_t = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;

                let ys = self
                    .model
                    .decoder
                    .forward(&tokens_t, &audio_features, i == 0)
                    .context("Decoder forward failed")?;
                drop(tokens_t);

                let seq_len = tokens.len();
                let logits_slice = ys.i((..1, seq_len - 1..))?;
                let logits = self
                    .model
                    .decoder
                    .final_linear(&logits_slice)?
                    .squeeze(0)?
                    .squeeze(0)?;
                drop(ys);
                drop(logits_slice);

                // Apply suppress tokens
                let logits = logits.broadcast_add(&self.suppress_tokens)?;

                // Greedy argmax — copy to CPU then drop GPU tensor
                let logits_v: Vec<f32> = logits.to_vec1()?;
                drop(logits);

                let next = logits_v
                    .iter()
                    .enumerate()
                    .max_by(|(_, a): &(usize, &f32), (_, b): &(usize, &f32)| a.total_cmp(b))
                    .map(|(idx, _)| idx as u32)
                    .unwrap_or(self.eot_token);

                Ok(next)
            })?;

            if next_token == self.eot_token {
                break;
            }

            tokens.push(next_token);
        }

        // Free encoder output now that decoding is done
        drop(audio_features);

        // Decode tokens to text, skipping special tokens
        let text_tokens: Vec<u32> = tokens
            .into_iter()
            .skip(4) // Skip SOT, language, transcribe, no_timestamps
            .collect();

        let text = self
            .tokenizer
            .decode(&text_tokens, true)
            .map_err(|e| anyhow::anyhow!("Token decoding failed: {}", e))?;

        Ok(text)
    }
}
