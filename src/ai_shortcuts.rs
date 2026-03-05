use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::Config;
use crate::input::{ClipboardManager, InputSimulator};
use crate::retry::{is_retryable_error, retry_with_backoff};

/// AI-powered text transformation shortcuts (like "Fix Grammar")
pub struct AiShortcuts {
    client: Client,
    config: Config,
}

/// Available AI shortcut actions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutAction {
    FixGrammar,
    MakeConcise,
    MakeFormal,
    MakeCasual,
    TranslateToSpanish,
    TranslateToFrench,
    Summarize,
    ExpandText,
}

impl ShortcutAction {
    /// Returns all available shortcut actions.
    pub fn all() -> &'static [ShortcutAction] {
        &[
            ShortcutAction::FixGrammar,
            ShortcutAction::MakeConcise,
            ShortcutAction::MakeFormal,
            ShortcutAction::MakeCasual,
            ShortcutAction::TranslateToSpanish,
            ShortcutAction::TranslateToFrench,
            ShortcutAction::Summarize,
            ShortcutAction::ExpandText,
        ]
    }

    pub fn prompt(&self) -> &'static str {
        match self {
            ShortcutAction::FixGrammar => {
                "Fix any grammar, spelling, and punctuation errors in the following text. \
                 Return only the corrected text without any explanations."
            }
            ShortcutAction::MakeConcise => {
                "Make the following text more concise while preserving its meaning. \
                 Return only the revised text without any explanations."
            }
            ShortcutAction::MakeFormal => {
                "Rewrite the following text in a formal, professional tone. \
                 Return only the revised text without any explanations."
            }
            ShortcutAction::MakeCasual => {
                "Rewrite the following text in a casual, friendly tone. \
                 Return only the revised text without any explanations."
            }
            ShortcutAction::TranslateToSpanish => {
                "Translate the following text to Spanish. \
                 Return only the translation without any explanations."
            }
            ShortcutAction::TranslateToFrench => {
                "Translate the following text to French. \
                 Return only the translation without any explanations."
            }
            ShortcutAction::Summarize => {
                "Summarize the following text in 1-2 sentences. \
                 Return only the summary without any explanations."
            }
            ShortcutAction::ExpandText => {
                "Expand and elaborate on the following text while maintaining its core message. \
                 Return only the expanded text without any explanations."
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShortcutAction::FixGrammar => "Fix Grammar",
            ShortcutAction::MakeConcise => "Make Concise",
            ShortcutAction::MakeFormal => "Make Formal",
            ShortcutAction::MakeCasual => "Make Casual",
            ShortcutAction::TranslateToSpanish => "Translate to Spanish",
            ShortcutAction::TranslateToFrench => "Translate to French",
            ShortcutAction::Summarize => "Summarize",
            ShortcutAction::ExpandText => "Expand",
        }
    }
}

impl AiShortcuts {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Execute an AI shortcut on the currently selected text.
    /// This copies the selection, sends to AI, and pastes the result.
    pub async fn execute_shortcut(&self, action: ShortcutAction) -> Result<()> {
        log::info!("Executing AI shortcut: {}", action.name());

        // Clipboard/input work must happen on the main thread (not Send).
        // We do it in blocking sections around the async AI call.
        let selected_text = tokio::task::spawn_blocking(|| -> Result<(String, Option<String>)> {
            let mut input = InputSimulator::new()?;
            let mut clipboard = ClipboardManager::new()?;

            let original_clipboard = clipboard.get_text().ok();
            let before = clipboard.get_text().unwrap_or_default();

            input.copy()?;

            // Poll clipboard until content changes (up to 500ms)
            let mut selected = before.clone();
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Ok(text) = clipboard.get_text() {
                    if text != before {
                        selected = text;
                        break;
                    }
                }
            }

            if selected.trim().is_empty() {
                anyhow::bail!("No text selected");
            }

            Ok((selected, original_clipboard))
        })
        .await
        .context("Spawn blocking failed")??;

        let (text, original_clipboard) = selected_text;
        log::info!("Selected text: {} characters", text.len());

        // Process with AI (async, Send-safe)
        let result = self.process_text(&text, action).await?;
        log::info!("AI result: {} characters", result.len());

        // Paste result back
        let original = original_clipboard.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut input = InputSimulator::new()?;
            let mut clipboard = ClipboardManager::new()?;

            clipboard.set_text(&result)?;
            input.paste()?;

            if let Some(orig) = original {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let _ = clipboard.set_text(&orig);
            }
            Ok(())
        })
        .await
        .context("Spawn blocking failed")??;

        Ok(())
    }

    /// Process text using the configured AI provider with retry.
    async fn process_text(&self, text: &str, action: ShortcutAction) -> Result<String> {
        let provider = &self.config.ai_shortcuts.provider;

        retry_with_backoff(
            3,
            Duration::from_millis(500),
            is_retryable_error,
            || async {
                match provider.as_str() {
                    "lmstudio" => self.process_local(text, action).await,
                    "openai" => self.process_openai(text, action).await,
                    "openrouter" => self.process_openrouter(text, action).await,
                    _ => {
                        log::warn!("Unknown AI provider '{}', falling back to local", provider);
                        self.process_local(text, action).await
                    }
                }
            },
        )
        .await
    }

    /// Process text using OpenAI API
    async fn process_openai(&self, text: &str, action: ShortcutAction) -> Result<String> {
        let api_key = self
            .config
            .get_ai_shortcut_api_key()
            .context("No API key configured for AI shortcuts")?;

        let request = OpenAiChatRequest {
            model: self.config.ai_shortcuts.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: action.prompt().to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            temperature: 0.3,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error {}: {}", status, error_text);
        }

        let result: OpenAiChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    /// Process text using OpenRouter API
    async fn process_openrouter(&self, text: &str, action: ShortcutAction) -> Result<String> {
        let api_key = self
            .config
            .get_ai_shortcut_api_key()
            .context("No API key configured for AI shortcuts")?;

        let request = OpenAiChatRequest {
            model: self.config.ai_shortcuts.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: action.prompt().to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            temperature: 0.3,
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://github.com/ottex")
            .header("X-Title", "Ottex Voice-to-Text")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error {}: {}", status, error_text);
        }

        let result: OpenAiChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    /// Process text using a local OpenAI-compatible server (LM Studio, etc.)
    async fn process_local(&self, text: &str, action: ShortcutAction) -> Result<String> {
        let base_url = &self.config.ai_shortcuts.local_url;
        if base_url.is_empty() {
            anyhow::bail!("Local server URL not configured for AI shortcuts");
        }

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let request = OpenAiChatRequest {
            model: self.config.ai_shortcuts.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: action.prompt().to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            temperature: 0.3,
        };

        log::info!("Sending AI shortcut to local server at {}", url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to local AI server")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Local AI server error {}: {}", status, error_text);
        }

        let result: OpenAiChatResponse = response
            .json()
            .await
            .context("Failed to parse local AI server response")?;

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }
}

// OpenAI/OpenRouter compatible API types
#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}
