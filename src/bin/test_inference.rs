use anyhow::Result;
use ottex::{EngineConfig, OttexEngine};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let rt = Runtime::new()?;
    rt.block_on(async {
        let config = EngineConfig {
            whisper_repo_id: "openai/whisper-large-v3".to_string(),
            whisper_revision: "main".to_string(),
        };

        let engine = OttexEngine::new(config);
        println!("Loading models...");
        let _ = engine.load_models().await.unwrap();
        
        // Force transcription using a fake audio payload directly via the supervisor
        let mut fake_audio = vec![0.0f32; 16000 * 3]; // 3 seconds
        for i in 0..fake_audio.len() {
            fake_audio[i] = ((i as f32) * 0.1).sin() * 0.5; // Sine wave to bypass silence check
        }

        println!("Transcribing fake audio...");
        match engine.inference_supervisor.transcribe(fake_audio).await {
            Ok(text) => println!("Transcribed: {}", text),
            Err(e) => println!("Error transcribing: {:?}", e),
        }
    });

    Ok(())
}
