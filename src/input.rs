use anyhow::{Context, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

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
        log::info!("Typing {} characters", text.len());

        // Small delay before typing to ensure focus is correct
        thread::sleep(Duration::from_millis(100));

        self.enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;

        log::info!("Finished typing");
        Ok(())
    }

    /// Simulate Ctrl+C (copy)
    pub fn copy(&mut self) -> Result<()> {
        log::debug!("Simulating Ctrl+C");

        self.enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press Ctrl: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Unicode('c'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press C: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release Ctrl: {:?}", e))?;

        thread::sleep(Duration::from_millis(100));

        Ok(())
    }

    /// Simulate Ctrl+V (paste)
    pub fn paste(&mut self) -> Result<()> {
        log::debug!("Simulating Ctrl+V");

        self.enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press Ctrl: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press V: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release Ctrl: {:?}", e))?;

        thread::sleep(Duration::from_millis(100));

        Ok(())
    }

    /// Simulate Ctrl+A (select all)
    #[allow(dead_code)]
    pub fn select_all(&mut self) -> Result<()> {
        log::debug!("Simulating Ctrl+A");

        self.enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press Ctrl: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Unicode('a'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press A: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        self.enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release Ctrl: {:?}", e))?;

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
        let clipboard = arboard::Clipboard::new()
            .context("Failed to access clipboard")?;

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
