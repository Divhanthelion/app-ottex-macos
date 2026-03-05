mod audio;
mod audio_processing;
mod config;
mod inference;
mod input;
mod retry;
mod transcription;

use anyhow::Result;
use std::sync::{Arc, Mutex};

// Setup uniffi scaffolding
uniffi::include_scaffolding!("ottex");

pub fn init_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .try_init();
}

pub struct EngineConfig {
    pub whisper_repo_id: String,
    pub whisper_revision: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OttexError {
    #[error("Setup failed: {0}")]
    SetupFailed(String),
    #[error("Recording failed: {0}")]
    RecordingFailed(String),
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("General error: {0}")]
    GeneralError(String),
}

impl From<anyhow::Error> for OttexError {
    fn from(err: anyhow::Error) -> Self {
        OttexError::GeneralError(err.to_string())
    }
}

pub struct OttexEngine {
    config: EngineConfig,
    recorder: Arc<Mutex<audio::AudioRecorder>>,
    pub inference_supervisor: Arc<inference::InferenceSupervisor>,
}

impl OttexEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            recorder: Arc::new(Mutex::new(audio::AudioRecorder::new())),
            inference_supervisor: Arc::new(inference::InferenceSupervisor::new()),
        }
    }

    pub async fn load_models(&self) -> Result<(), OttexError> {
        let local_config = config::LocalModelConfig {
            model_cache_dir: "~/.cache/ottex/models".to_string(),
            repo_id: self.config.whisper_repo_id.clone(),
            revision: self.config.whisper_revision.clone(),
            quantization: config::QuantizationMode::F32,
            temperature: 0.0,
        };

        // Load Whisper
        self.inference_supervisor
            .load_model(local_config)
            .await
            .map_err(|e| OttexError::SetupFailed(format!("Whisper Error: {}", e)))?;
        
        Ok(())
    }

    pub fn start_recording(&self) -> Result<(), OttexError> {
        let mut recorder_guard = self.recorder.lock().unwrap();
        recorder_guard
            .start_recording()
            .map_err(|e| OttexError::RecordingFailed(e.to_string()))?;
        Ok(())
    }

    pub async fn stop_and_transcribe(&self) -> Result<String, OttexError> {
        let audio_data = {
            let mut recorder_guard = self.recorder.lock().unwrap();
            recorder_guard.stop().map_err(|e| OttexError::RecordingFailed(e.to_string()))?
        };

        if !audio_data.has_audio() {
            return Ok("".to_string());
        }

        let mut resampler = audio_processing::AudioResampler::new(audio_data.sample_rate, audio_data.channels)
            .map_err(|e| OttexError::InferenceFailed(format!("{:?}", e)))?;
        let audio_f32_16khz = resampler.resample_i16_to_f32_16khz(&audio_data.samples)
            .map_err(|e| OttexError::InferenceFailed(format!("{:?}", e)))?;
        
        let text = self.inference_supervisor
            .transcribe(audio_f32_16khz)
            .await
            .map_err(|e| OttexError::InferenceFailed(format!("{:?}", e)))?;
        
        Ok(text)
    }

    /// Takes the transcribed text and automatically types it using keyboard simulation
    pub fn type_text(&self, text: String) -> Result<(), OttexError> {
        if text.trim().is_empty() { return Ok(()); }
        
        let mut simulator = input::InputSimulator::new()
            .map_err(|e| OttexError::GeneralError(e.to_string()))?;
            
        simulator.type_text(&text)
            .map_err(|e| OttexError::GeneralError(e.to_string()))?;
            
        Ok(())
    }
}
