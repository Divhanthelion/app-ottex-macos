use anyhow::{Context, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Return the platform modifier key (Cmd on macOS, Ctrl elsewhere).
fn platform_modifier() -> Key {
    if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    }
}

/// Input simulator for typing text and keyboard shortcuts
pub struct InputSimulator {
    enigo: Enigo,
}

impl InputSimulator {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Failed to create Enigo: {:?}", e))?;

        Ok(Self { enigo })
    }

    /// Type the given text as keyboard input
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        log::info!("Typing {} characters via clipboard", text.len());

        // Small delay before typing to ensure focus is correct
        thread::sleep(Duration::from_millis(100));

        // Use clipboard paste instead of simulated typing for better reliability across apps
        if let Ok(mut clipboard) = ClipboardManager::new() {
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard: {:?}", e);
                // Fallback to enigo.text
                self.enigo
                    .text(text)
                    .map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;
            } else {
                // Wait for clipboard to sync
                thread::sleep(Duration::from_millis(50));
                self.paste()?;
            }
        } else {
            // Fallback
            self.enigo
                .text(text)
                .map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;
        }

        log::info!("Finished typing");
        Ok(())
    }

    /// Simulate Copy (Cmd+C on macOS, Ctrl+C elsewhere)
    pub fn copy(&mut self) -> Result<()> {
        let modifier = platform_modifier();
        log::debug!("Simulating Copy");

        self.enigo
            .key(modifier, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press modifier: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Unicode('c'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press C: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(modifier, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release modifier: {:?}", e))?;

        thread::sleep(Duration::from_millis(100));

        Ok(())
    }

    /// Simulate Paste (Cmd+V on macOS, Ctrl+V elsewhere)
    pub fn paste(&mut self) -> Result<()> {
        let modifier = platform_modifier();
        log::debug!("Simulating Paste");

        self.enigo
            .key(modifier, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press modifier: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press V: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(modifier, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release modifier: {:?}", e))?;

        thread::sleep(Duration::from_millis(100));

        Ok(())
    }

    /// Simulate Select All (Cmd+A on macOS, Ctrl+A elsewhere)
    #[allow(dead_code)]
    pub fn select_all(&mut self) -> Result<()> {
        let modifier = platform_modifier();
        log::debug!("Simulating Select All");

        self.enigo
            .key(modifier, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press modifier: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Unicode('a'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press A: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(modifier, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release modifier: {:?}", e))?;

        thread::sleep(Duration::from_millis(100));

        Ok(())
    }
}

/// Clipboard operations using arboard
pub struct ClipboardManager {
    clipboard: arboard::Clipboard,
}

impl ClipboardManager {
    pub fn new() -> Result<Self> {
        let clipboard = arboard::Clipboard::new().context("Failed to access clipboard")?;

        Ok(Self { clipboard })
    }

    /// Get text from clipboard
    pub fn get_text(&mut self) -> Result<String> {
        self.clipboard
            .get_text()
            .context("Failed to get text from clipboard")
    }

    /// Set text to clipboard
    pub fn set_text(&mut self, text: &str) -> Result<()> {
        self.clipboard
            .set_text(text)
            .context("Failed to set text to clipboard")
    }
}
