use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::input::{ClipboardManager, InputSimulator};

/// AI-powered text transformation shortcuts (like "Fix Grammar")
pub struct AiShortcuts {
    client: Client,
    config: Config,
}

/// Available AI shortcut actions
#[derive(Debug, Clone, Copy)]
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

    /// Execute an AI shortcut on the currently selected text
    /// This copies the selection, sends to AI, and pastes the result
    pub async fn execute_shortcut(&self, action: ShortcutAction) -> Result<()> {
        log::info!("Executing AI shortcut: {}", action.name());

        // Create input simulator and clipboard manager
        let mut input = InputSimulator::new()?;
        let mut clipboard = ClipboardManager::new()?;

        // Save current clipboard content
        let original_clipboard = clipboard.get_text().ok();

        // Copy selected text
        input.copy()?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Get the copied text
        let selected_text = clipboard.get_text()
            .context("No text selected or clipboard empty")?;

        if selected_text.trim().is_empty() {
            anyhow::bail!("No text selected");
        }

        log::info!("Selected text: {} characters", selected_text.len());

        // Process with AI
        let result = self.process_text(&selected_text, action).await?;

        log::info!("AI result: {} characters", result.len());

        // Put result in clipboard
        clipboard.set_text(&result)?;

        // Paste the result (replacing the selection)
        input.paste()?;

        // Restore original clipboard after a delay
        if let Some(original) = original_clipboard {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = clipboard.set_text(&original);
        }

        Ok(())
    }

    /// Process text using the configured AI provider
    async fn process_text(&self, text: &str, action: ShortcutAction) -> Result<String> {
        let provider = &self.config.ai_shortcuts.provider;

        match provider.as_str() {
            "openai" => self.process_openai(text, action).await,
            "openrouter" => self.process_openrouter(text, action).await,
            _ => {
                log::warn!("Unknown AI provider '{}', falling back to OpenAI", provider);
                self.process_openai(text, action).await
            }
        }
    }

    /// Process text using OpenAI API
    async fn process_openai(&self, text: &str, action: ShortcutAction) -> Result<String> {
        let api_key = self.config.get_ai_shortcut_api_key()
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
        let api_key = self.config.get_ai_shortcut_api_key()
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
