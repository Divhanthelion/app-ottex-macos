use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub transcription: TranscriptionConfig,
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default)]
    pub ai_shortcuts: AiShortcutConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Primary provider: "google" or "openai" or "openrouter"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Google Cloud Speech-to-Text API key
    #[serde(default)]
    pub google_api_key: String,
    /// OpenAI API key for Whisper
    #[serde(default)]
    pub openai_api_key: String,
    /// OpenRouter API key
    #[serde(default)]
    pub openrouter_api_key: String,
    /// Language code (default: en-US)
    #[serde(default = "default_language")]
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Hotkey for voice recording (default: Alt+Space)
    #[serde(default = "default_record_hotkey")]
    pub record: String,
    /// Hotkey for AI shortcuts (default: Ctrl+Shift+R)
    #[serde(default = "default_shortcut_hotkey")]
    pub ai_shortcut: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiShortcutConfig {
    /// API key for AI completions (uses openai_api_key or openrouter_api_key by default)
    #[serde(default)]
    pub api_key: String,
    /// Provider for AI shortcuts: "openai" or "openrouter"
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    /// Model to use for AI shortcuts
    #[serde(default = "default_ai_model")]
    pub model: String,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_language() -> String {
    "en-US".to_string()
}

fn default_record_hotkey() -> String {
    "Alt+Space".to_string()
}

fn default_shortcut_hotkey() -> String {
    "Ctrl+Shift+R".to_string()
}

fn default_ai_provider() -> String {
    "openai".to_string()
}

fn default_ai_model() -> String {
    "gpt-4o-mini".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            transcription: TranscriptionConfig::default(),
            hotkeys: HotkeyConfig::default(),
            ai_shortcuts: AiShortcutConfig::default(),
        }
    }
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            google_api_key: String::new(),
            openai_api_key: String::new(),
            openrouter_api_key: String::new(),
            language: default_language(),
        }
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            record: default_record_hotkey(),
            ai_shortcut: default_shortcut_hotkey(),
        }
    }
}

impl Default for AiShortcutConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            provider: default_ai_provider(),
            model: default_ai_model(),
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "ottex", "Ottex")
            .context("Failed to get project directories")?;
        Ok(proj_dirs.config_dir().to_path_buf())
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            log::info!("Config file not found, creating default at {:?}", config_path);
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = Self::config_dir()?;

        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .context("Failed to write config file")?;

        log::info!("Config saved to {:?}", config_path);
        Ok(())
    }

    /// Get the effective API key for transcription based on provider
    pub fn get_transcription_api_key(&self) -> Option<&str> {
        match self.transcription.provider.as_str() {
            "google" => {
                if self.transcription.google_api_key.is_empty() {
                    None
                } else {
                    Some(&self.transcription.google_api_key)
                }
            }
            "openai" => {
                if self.transcription.openai_api_key.is_empty() {
                    None
                } else {
                    Some(&self.transcription.openai_api_key)
                }
            }
            "openrouter" => {
                if self.transcription.openrouter_api_key.is_empty() {
                    None
                } else {
                    Some(&self.transcription.openrouter_api_key)
                }
            }
            _ => None,
        }
    }

    /// Get the effective API key for AI shortcuts
    pub fn get_ai_shortcut_api_key(&self) -> Option<&str> {
        // First check dedicated AI shortcut key
        if !self.ai_shortcuts.api_key.is_empty() {
            return Some(&self.ai_shortcuts.api_key);
        }

        // Fall back to provider-specific key
        match self.ai_shortcuts.provider.as_str() {
            "openai" => {
                if self.transcription.openai_api_key.is_empty() {
                    None
                } else {
                    Some(&self.transcription.openai_api_key)
                }
            }
            "openrouter" => {
                if self.transcription.openrouter_api_key.is_empty() {
                    None
                } else {
                    Some(&self.transcription.openrouter_api_key)
                }
            }
            _ => None,
        }
    }
}
