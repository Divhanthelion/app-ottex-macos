use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const VALID_TRANSCRIPTION_PROVIDERS: &[&str] =
    &["embedded", "lmstudio", "google", "openai", "openrouter"];
const VALID_AI_PROVIDERS: &[&str] = &["lmstudio", "openai", "openrouter"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub transcription: TranscriptionConfig,
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default)]
    pub ai_shortcuts: AiShortcutConfig,
}

/// Transcription strategy when embedded model is available
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStrategy {
    /// Only use embedded model, fail if unavailable
    EmbeddedOnly,
    /// Try embedded first, fall back to cloud providers
    #[default]
    EmbeddedFirst,
    /// Only use cloud/local-server providers
    CloudOnly,
}

/// Quantization mode for embedded model weights
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantizationMode {
    #[default]
    F32,
    F16,
}

/// Configuration for the locally-embedded Whisper model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelConfig {
    /// Directory to cache downloaded model files
    #[serde(default = "default_model_cache_dir")]
    pub model_cache_dir: String,
    /// Hugging Face repo ID for the model
    #[serde(default = "default_repo_id")]
    pub repo_id: String,
    /// Model revision/branch
    #[serde(default = "default_revision")]
    pub revision: String,
    /// Weight quantization mode
    #[serde(default)]
    pub quantization: QuantizationMode,
    /// Temperature for decoding (0.0 = greedy)
    #[serde(default)]
    pub temperature: f64,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            model_cache_dir: default_model_cache_dir(),
            repo_id: default_repo_id(),
            revision: default_revision(),
            quantization: QuantizationMode::default(),
            temperature: 0.0,
        }
    }
}

fn default_model_cache_dir() -> String {
    directories::ProjectDirs::from("com", "ottex", "Ottex")
        .map(|d| d.cache_dir().join("models").to_string_lossy().to_string())
        .unwrap_or_else(|| "~/.cache/ottex/models".to_string())
}

fn default_repo_id() -> String {
    "openai/whisper-large-v3-turbo".to_string()
}

fn default_revision() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Primary provider: "embedded", "lmstudio", "google", "openai", or "openrouter"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Transcription strategy for embedded model fallback behavior
    #[serde(default)]
    pub strategy: TranscriptionStrategy,
    /// Configuration for the embedded Whisper model
    #[serde(default)]
    pub embedded: LocalModelConfig,
    /// Base URL for local server (LM Studio, Ollama, etc.)
    #[serde(default = "default_local_url")]
    pub local_url: String,
    /// Model name for local transcription (sent in multipart form)
    #[serde(default = "default_local_transcription_model")]
    pub local_model: String,
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
    /// Provider for AI shortcuts: "lmstudio", "openai", or "openrouter"
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    /// Model to use for AI shortcuts
    #[serde(default = "default_ai_model")]
    pub model: String,
    /// Base URL for local server (LM Studio, Ollama, etc.)
    #[serde(default = "default_local_url")]
    pub local_url: String,
}

fn default_provider() -> String {
    "embedded".to_string()
}

fn default_local_url() -> String {
    "http://localhost:1234/v1".to_string()
}

fn default_local_transcription_model() -> String {
    "qwen3-asr-1.7b".to_string()
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
    "lmstudio".to_string()
}

fn default_ai_model() -> String {
    "qwen3-8b".to_string()
}

// Config derives Default since all fields have their own Default impls.

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            strategy: TranscriptionStrategy::default(),
            embedded: LocalModelConfig::default(),
            local_url: default_local_url(),
            local_model: default_local_transcription_model(),
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
            local_url: default_local_url(),
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
            log::info!(
                "Config file not found, creating default at {:?}",
                config_path
            );
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path).context("Failed to read config file")?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = Self::config_dir()?;

        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config file")?;

        log::info!("Config saved to {:?}", config_path);
        Ok(())
    }

    /// Get the effective API key for transcription based on provider.
    /// Returns `None` for local providers (no key needed) or if key is missing.
    pub fn get_transcription_api_key(&self) -> Option<&str> {
        match self.transcription.provider.as_str() {
            "embedded" | "lmstudio" => None, // local, no key needed
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

    /// Validate configuration and return a list of warnings.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if !VALID_TRANSCRIPTION_PROVIDERS.contains(&self.transcription.provider.as_str()) {
            warnings.push(format!(
                "Unknown transcription provider '{}', expected one of {:?}",
                self.transcription.provider, VALID_TRANSCRIPTION_PROVIDERS
            ));
        }

        if !VALID_AI_PROVIDERS.contains(&self.ai_shortcuts.provider.as_str()) {
            warnings.push(format!(
                "Unknown AI shortcuts provider '{}', expected one of {:?}",
                self.ai_shortcuts.provider, VALID_AI_PROVIDERS
            ));
        }

        // Local providers don't need API keys
        if self.transcription.provider != "lmstudio"
            && self.transcription.provider != "embedded"
            && self.get_transcription_api_key().is_none()
        {
            warnings.push(format!(
                "No API key configured for transcription provider '{}'",
                self.transcription.provider
            ));
        }

        if self.ai_shortcuts.provider != "lmstudio" && self.get_ai_shortcut_api_key().is_none() {
            warnings.push(format!(
                "No API key configured for AI shortcuts provider '{}'",
                self.ai_shortcuts.provider
            ));
        }

        warnings
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.transcription.provider, "embedded");
        assert_eq!(config.transcription.local_url, "http://localhost:1234/v1");
        assert_eq!(config.transcription.language, "en-US");
        assert_eq!(config.hotkeys.record, "Alt+Space");
        assert_eq!(config.hotkeys.ai_shortcut, "Ctrl+Shift+R");
        assert_eq!(config.ai_shortcuts.provider, "lmstudio");
        assert_eq!(config.ai_shortcuts.model, "qwen3-8b");
    }

    #[test]
    fn test_validation_default_no_warnings_for_local() {
        let config = Config::default();
        let warnings = config.validate();
        // Default config uses lmstudio — no API key warnings
        assert!(!warnings.iter().any(|w| w.contains("No API key")));
    }

    #[test]
    fn test_validation_cloud_warns_about_keys() {
        let mut config = Config::default();
        config.transcription.provider = "openai".to_string();
        config.ai_shortcuts.provider = "openai".to_string();
        let warnings = config.validate();
        assert!(warnings.iter().any(|w| w.contains("No API key")));
    }

    #[test]
    fn test_validation_invalid_provider() {
        let mut config = Config::default();
        config.transcription.provider = "invalid".to_string();
        let warnings = config.validate();
        assert!(warnings
            .iter()
            .any(|w| w.contains("Unknown transcription provider")));
    }

    #[test]
    fn test_get_transcription_api_key() {
        let mut config = Config::default();
        // lmstudio returns None (no key needed)
        assert!(config.get_transcription_api_key().is_none());

        // Switch to openai provider
        config.transcription.provider = "openai".to_string();
        assert!(config.get_transcription_api_key().is_none());

        config.transcription.openai_api_key = "sk-test".to_string();
        assert_eq!(config.get_transcription_api_key(), Some("sk-test"));
    }

    #[test]
    fn test_get_ai_shortcut_api_key_fallback() {
        let mut config = Config::default();
        // Switch to openai to test key resolution
        config.ai_shortcuts.provider = "openai".to_string();
        config.transcription.openai_api_key = "sk-shared".to_string();
        // Should fall back to transcription key
        assert_eq!(config.get_ai_shortcut_api_key(), Some("sk-shared"));

        // Dedicated key takes priority
        config.ai_shortcuts.api_key = "sk-dedicated".to_string();
        assert_eq!(config.get_ai_shortcut_api_key(), Some("sk-dedicated"));
    }

    #[test]
    fn test_roundtrip_toml() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(
            config.transcription.provider,
            deserialized.transcription.provider
        );
        assert_eq!(config.hotkeys.record, deserialized.hotkeys.record);
    }
}
