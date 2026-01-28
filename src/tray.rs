use anyhow::{Context, Result};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, MenuId, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};

/// System tray manager
pub struct TrayManager {
    #[allow(dead_code)]
    tray_icon: TrayIcon,
    pub quit_item_id: MenuId,
    pub settings_item_id: MenuId,
}

impl TrayManager {
    pub fn new() -> Result<Self> {
        // Create menu
        let menu = Menu::new();

        let settings_item = MenuItem::new("Settings...", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let settings_item_id = settings_item.id().clone();
        let quit_item_id = quit_item.id().clone();

        menu.append(&settings_item)
            .context("Failed to add settings item")?;
        menu.append(&PredefinedMenuItem::separator())
            .context("Failed to add separator")?;
        menu.append(&quit_item)
            .context("Failed to add quit item")?;

        // Create tray icon
        let icon = create_icon()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Ottex - Voice to Text")
            .with_icon(icon)
            .build()
            .context("Failed to create tray icon")?;

        log::info!("Tray icon created successfully");

        Ok(Self {
            tray_icon,
            quit_item_id,
            settings_item_id,
        })
    }

    /// Check for menu events
    pub fn check_menu_event(&self) -> Option<MenuId> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            Some(event.id)
        } else {
            None
        }
    }
}

/// Create a simple icon for the tray
fn create_icon() -> Result<tray_icon::Icon> {
    // Create a simple 32x32 RGBA icon (microphone-like shape)
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    // Fill with a mic-like icon (simplified)
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;

            // Center coordinates
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let px = x as f32;
            let py = y as f32;

            // Draw mic body (oval)
            let in_mic_body = {
                let rx = 8.0;
                let ry = 12.0;
                let cy_mic = cy - 4.0;
                ((px - cx) / rx).powi(2) + ((py - cy_mic) / ry).powi(2) <= 1.0
            };

            // Draw mic stand (line at bottom)
            let in_stand = {
                let stand_width = 3.0;
                let stand_top = cy + 8.0;
                let stand_bottom = cy + 14.0;
                (px - cx).abs() < stand_width && py >= stand_top && py <= stand_bottom
            };

            // Draw mic base (horizontal line)
            let in_base = {
                let base_y = cy + 13.0;
                let base_half_width = 8.0;
                (px - cx).abs() < base_half_width && (py - base_y).abs() < 2.0
            };

            if in_mic_body || in_stand || in_base {
                // White with full opacity
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            } else {
                // Transparent
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 0;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size, size)
        .context("Failed to create icon from RGBA data")
}
