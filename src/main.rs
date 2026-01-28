#![windows_subsystem = "windows"]

mod ai_shortcuts;
mod app;
mod audio;
mod config;
mod hotkey;
mod input;
mod transcription;
mod tray;

use anyhow::Result;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("Ottex Voice-to-Text starting...");

    // Create and run the application
    let mut app = app::App::new()?;
    app.run()?;

    Ok(())
}
