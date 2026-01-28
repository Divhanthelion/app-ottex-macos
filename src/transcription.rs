use anyhow::{Context, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Transcription service that handles multiple providers
pub struct TranscriptionService {
    client: Client,
    config: Config,
}

impl TranscriptionService {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Transcribe audio using the configured provider
    pub async fn transcribe(&self, wav_bytes: &[u8]) -> Result<String> {
        let provider = &self.config.transcription.provider;

        log::info!("Transcribing with provider: {}", provider);

        let result = match provider.as_str() {
            "google" => self.transcribe_google(wav_bytes).await,
            "openai" => self.transcribe_openai(wav_bytes).await,
            "openrouter" => self.transcribe_openrouter(wav_bytes).await,
            _ => {
                log::warn!("Unknown provider '{}', falling back to OpenAI", provider);
                self.transcribe_openai(wav_bytes).await
            }
        };

        // If primary fails, try fallback
        match result {
            Ok(text) => Ok(text),
            Err(e) => {
                log::warn!("Primary transcription failed: {}, trying fallback", e);
                self.transcribe_fallback(wav_bytes).await
            }
        }
    }

    /// Try fallback providers
    async fn transcribe_fallback(&self, wav_bytes: &[u8]) -> Result<String> {
        let primary = &self.config.transcription.provider;

        // Try providers in order, skipping the primary
        let fallbacks = ["openai", "openrouter", "google"];

        for provider in fallbacks {
            if provider == primary {
                continue;
            }

            let result = match provider {
                "google" if !self.config.transcription.google_api_key.is_empty() => {
                    self.transcribe_google(wav_bytes).await
                }
                "openai" if !self.config.transcription.openai_api_key.is_empty() => {
                    self.transcribe_openai(wav_bytes).await
                }
                "openrouter" if !self.config.transcription.openrouter_api_key.is_empty() => {
                    self.transcribe_openrouter(wav_bytes).await
                }
                _ => continue,
            };

            if let Ok(text) = result {
                log::info!("Fallback to {} succeeded", provider);
                return Ok(text);
            }
        }

        anyhow::bail!("All transcription providers failed")
    }

    /// Transcribe using Google Cloud Speech-to-Text API
    async fn transcribe_google(&self, wav_bytes: &[u8]) -> Result<String> {
        let api_key = &self.config.transcription.google_api_key;
        if api_key.is_empty() {
            anyhow::bail!("Google API key not configured");
        }

        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(wav_bytes);

        let request_body = GoogleSpeechRequest {
            config: GoogleSpeechConfig {
                encoding: "LINEAR16".to_string(),
                sample_rate_hertz: 48000, // Will be overridden by WAV header
                language_code: self.config.transcription.language.clone(),
                enable_automatic_punctuation: true,
            },
            audio: GoogleSpeechAudio {
                content: audio_base64,
            },
        };

        let url = format!(
            "https://speech.googleapis.com/v1/speech:recognize?key={}",
            api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google Speech API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Google API error {}: {}", status, error_text);
        }

        let result: GoogleSpeechResponse = response
            .json()
            .await
            .context("Failed to parse Google Speech response")?;

        let transcript = result
            .results
            .into_iter()
            .flat_map(|r| r.alternatives)
            .map(|a| a.transcript)
            .collect::<Vec<_>>()
            .join(" ");

        Ok(transcript)
    }

    /// Transcribe using OpenAI Whisper API
    async fn transcribe_openai(&self, wav_bytes: &[u8]) -> Result<String> {
        let api_key = &self.config.transcription.openai_api_key;
        if api_key.is_empty() {
            anyhow::bail!("OpenAI API key not configured");
        }

        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .text("language", "en")
            .part(
                "file",
                reqwest::multipart::Part::bytes(wav_bytes.to_vec())
                    .file_name("recording.wav")
                    .mime_str("audio/wav")?,
            );

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error {}: {}", status, error_text);
        }

        let result: OpenAiTranscriptionResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        Ok(result.text)
    }

    /// Transcribe using OpenRouter API (via chat completion with audio)
    async fn transcribe_openrouter(&self, wav_bytes: &[u8]) -> Result<String> {
        let api_key = &self.config.transcription.openrouter_api_key;
        if api_key.is_empty() {
            anyhow::bail!("OpenRouter API key not configured");
        }

        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(wav_bytes);

        let request_body = OpenRouterAudioRequest {
            model: "openai/whisper-large-v3".to_string(),
            messages: vec![OpenRouterMessage {
                role: "user".to_string(),
                content: vec![
                    OpenRouterContent::Text {
                        r#type: "text".to_string(),
                        text: "Please transcribe this audio accurately.".to_string(),
                    },
                    OpenRouterContent::Audio {
                        r#type: "input_audio".to_string(),
                        input_audio: OpenRouterAudioData {
                            data: audio_base64,
                            format: "wav".to_string(),
                        },
                    },
                ],
            }],
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://github.com/ottex")
            .header("X-Title", "Ottex Voice-to-Text")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error {}: {}", status, error_text);
        }

        let result: OpenRouterResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;

        let transcript = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(transcript)
    }
}

// Google Speech-to-Text API types
#[derive(Serialize)]
struct GoogleSpeechRequest {
    config: GoogleSpeechConfig,
    audio: GoogleSpeechAudio,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleSpeechConfig {
    encoding: String,
    sample_rate_hertz: u32,
    language_code: String,
    enable_automatic_punctuation: bool,
}

#[derive(Serialize)]
struct GoogleSpeechAudio {
    content: String,
}

#[derive(Deserialize)]
struct GoogleSpeechResponse {
    #[serde(default)]
    results: Vec<GoogleSpeechResult>,
}

#[derive(Deserialize)]
struct GoogleSpeechResult {
    #[serde(default)]
    alternatives: Vec<GoogleSpeechAlternative>,
}

#[derive(Deserialize)]
struct GoogleSpeechAlternative {
    transcript: String,
}

// OpenAI API types
#[derive(Deserialize)]
struct OpenAiTranscriptionResponse {
    text: String,
}

// OpenRouter API types
#[derive(Serialize)]
struct OpenRouterAudioRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: Vec<OpenRouterContent>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenRouterContent {
    Text {
        r#type: String,
        text: String,
    },
    Audio {
        r#type: String,
        input_audio: OpenRouterAudioData,
    },
}

#[derive(Serialize)]
struct OpenRouterAudioData {
    data: String,
    format: String,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
}

#[derive(Deserialize)]
struct OpenRouterResponseMessage {
    content: String,
}
