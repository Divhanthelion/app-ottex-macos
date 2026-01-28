use anyhow::{Context, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};

/// Hotkey manager for registering and handling global hotkeys
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    record_hotkey_id: u32,
    shortcut_hotkey_id: u32,
}

/// Hotkey event types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotkeyEvent {
    RecordPressed,
    RecordReleased,
    ShortcutPressed,
    ShortcutReleased,
}

impl HotkeyManager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .context("Failed to create hotkey manager")?;

        // Register Alt+Space for recording
        let record_hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
        manager
            .register(record_hotkey)
            .context("Failed to register record hotkey (Alt+Space)")?;

        log::info!("Registered record hotkey: Alt+Space (id: {})", record_hotkey.id());

        // Register Ctrl+Shift+R for AI shortcuts
        let shortcut_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR);
        manager
            .register(shortcut_hotkey)
            .context("Failed to register shortcut hotkey (Ctrl+Shift+R)")?;

        log::info!(
            "Registered AI shortcut hotkey: Ctrl+Shift+R (id: {})",
            shortcut_hotkey.id()
        );

        Ok(Self {
            manager,
            record_hotkey_id: record_hotkey.id(),
            shortcut_hotkey_id: shortcut_hotkey.id(),
        })
    }

    /// Check for hotkey events (non-blocking)
    pub fn check_event(&self) -> Option<HotkeyEvent> {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            log::debug!("Hotkey event: id={}, state={:?}", event.id, event.state);

            if event.id == self.record_hotkey_id {
                match event.state {
                    HotKeyState::Pressed => Some(HotkeyEvent::RecordPressed),
                    HotKeyState::Released => Some(HotkeyEvent::RecordReleased),
                }
            } else if event.id == self.shortcut_hotkey_id {
                match event.state {
                    HotKeyState::Pressed => Some(HotkeyEvent::ShortcutPressed),
                    HotKeyState::Released => Some(HotkeyEvent::ShortcutReleased),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Unregister all hotkeys
    #[allow(dead_code)]
    pub fn unregister_all(&self) -> Result<()> {
        let record_hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
        self.manager
            .unregister(record_hotkey)
            .context("Failed to unregister record hotkey")?;

        let shortcut_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR);
        self.manager
            .unregister(shortcut_hotkey)
            .context("Failed to unregister shortcut hotkey")?;

        Ok(())
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        // Try to unregister hotkeys on drop
        let _ = self.unregister_all();
    }
}
