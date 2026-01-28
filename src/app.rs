use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::ai_shortcuts::{AiShortcuts, ShortcutAction};
use crate::audio::{AudioRecorder, RecordingHandle};
use crate::config::Config;
use crate::hotkey::{HotkeyEvent, HotkeyManager};
use crate::input::InputSimulator;
use crate::transcription::TranscriptionService;
use crate::tray::TrayManager;

/// Application state
pub struct App {
    config: Config,
    tray: TrayManager,
    hotkey_manager: HotkeyManager,
    audio_recorder: AudioRecorder,
    transcription: TranscriptionService,
    ai_shortcuts: AiShortcuts,
    runtime: Runtime,
    is_recording: bool,
    recording_handle: Option<RecordingHandle>,
    running: Arc<AtomicBool>,
}

impl App {
    pub fn new() -> Result<Self> {
        // Load configuration
        let config = Config::load()?;
        log::info!("Configuration loaded");

        // Create async runtime
        let runtime = Runtime::new()?;

        // Initialize components
        let tray = TrayManager::new()?;
        let hotkey_manager = HotkeyManager::new()?;
        let audio_recorder = AudioRecorder::new()?;
        let transcription = TranscriptionService::new(config.clone());
        let ai_shortcuts = AiShortcuts::new(config.clone());

        Ok(Self {
            config,
            tray,
            hotkey_manager,
            audio_recorder,
            transcription,
            ai_shortcuts,
            runtime,
            is_recording: false,
            recording_handle: None,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Run the main event loop
    pub fn run(&mut self) -> Result<()> {
        log::info!("Starting Ottex application");
        log::info!("Press Alt+Space to start/stop recording");
        log::info!("Press Ctrl+Shift+R to apply AI shortcuts to selected text");

        // Check API key configuration
        if self.config.get_transcription_api_key().is_none() {
            log::warn!(
                "No transcription API key configured! Edit the config file at {:?}",
                Config::config_path()?
            );
        }

        while self.running.load(Ordering::Relaxed) {
            // Process tray menu events
            if let Some(menu_id) = self.tray.check_menu_event() {
                if menu_id == self.tray.quit_item_id {
                    log::info!("Quit requested from tray menu");
                    self.running.store(false, Ordering::Relaxed);
                    break;
                } else if menu_id == self.tray.settings_item_id {
                    self.open_settings()?;
                }
            }

            // Process hotkey events
            if let Some(event) = self.hotkey_manager.check_event() {
                self.handle_hotkey_event(event)?;
            }

            // Small sleep to prevent busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        log::info!("Application shutting down");
        Ok(())
    }

    fn handle_hotkey_event(&mut self, event: HotkeyEvent) -> Result<()> {
        match event {
            HotkeyEvent::RecordPressed => {
                if !self.is_recording {
                    self.start_recording()?;
                }
            }
            HotkeyEvent::RecordReleased => {
                if self.is_recording {
                    self.stop_recording_and_transcribe()?;
                }
            }
            HotkeyEvent::ShortcutPressed => {
                self.execute_ai_shortcut()?;
            }
            HotkeyEvent::ShortcutReleased => {
                // No action needed on release
            }
        }
        Ok(())
    }

    fn start_recording(&mut self) -> Result<()> {
        log::info!("Starting recording...");

        let handle = self.audio_recorder.start_recording()?;
        self.recording_handle = Some(handle);
        self.is_recording = true;

        log::info!("Recording started - release Alt+Space to stop");
        Ok(())
    }

    fn stop_recording_and_transcribe(&mut self) -> Result<()> {
        log::info!("Stopping recording...");

        let handle = self.recording_handle.take();
        self.is_recording = false;

        let Some(handle) = handle else {
            log::warn!("No active recording to stop");
            return Ok(());
        };

        // Stop recording and get audio data
        let audio = handle.stop()?;

        if !audio.has_audio() {
            log::warn!("Recording too short or silent, skipping transcription");
            return Ok(());
        }

        // Encode to WAV
        let wav_bytes = audio.to_wav()?;

        // Transcribe asynchronously
        log::info!("Transcribing audio...");

        let transcription = self.transcription.clone_service();
        let result = self.runtime.block_on(async {
            transcription.transcribe(&wav_bytes).await
        });

        match result {
            Ok(text) => {
                if text.trim().is_empty() {
                    log::warn!("Transcription returned empty text");
                    return Ok(());
                }

                log::info!("Transcription result: {}", text);

                // Type the transcribed text
                let mut input = InputSimulator::new()?;
                input.type_text(&text)?;
            }
            Err(e) => {
                log::error!("Transcription failed: {}", e);
            }
        }

        Ok(())
    }

    fn execute_ai_shortcut(&mut self) -> Result<()> {
        log::info!("Executing AI shortcut (Fix Grammar)...");

        // For now, default to Fix Grammar
        // A full implementation would show a selection menu
        let action = ShortcutAction::FixGrammar;

        let shortcuts = self.ai_shortcuts.clone_service();
        let result = self.runtime.block_on(async {
            shortcuts.execute_shortcut(action).await
        });

        match result {
            Ok(()) => {
                log::info!("AI shortcut completed successfully");
            }
            Err(e) => {
                log::error!("AI shortcut failed: {}", e);
            }
        }

        Ok(())
    }

    fn open_settings(&self) -> Result<()> {
        let config_path = Config::config_path()?;
        log::info!("Opening config file: {:?}", config_path);

        // Open the config file with the default text editor
        #[cfg(windows)]
        {
            std::process::Command::new("notepad")
                .arg(&config_path)
                .spawn()?;
        }

        #[cfg(not(windows))]
        {
            // On non-Windows, try xdg-open or open
            if cfg!(target_os = "macos") {
                std::process::Command::new("open")
                    .arg(&config_path)
                    .spawn()?;
            } else {
                std::process::Command::new("xdg-open")
                    .arg(&config_path)
                    .spawn()?;
            }
        }

        Ok(())
    }
}

// Helper trait to clone services for async operations
trait CloneService {
    fn clone_service(&self) -> Self;
}

impl CloneService for TranscriptionService {
    fn clone_service(&self) -> Self {
        TranscriptionService::new(Config::load().unwrap_or_default())
    }
}

impl CloneService for AiShortcuts {
    fn clone_service(&self) -> Self {
        AiShortcuts::new(Config::load().unwrap_or_default())
    }
}
