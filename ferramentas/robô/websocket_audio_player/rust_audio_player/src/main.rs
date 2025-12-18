mod audio;
mod config;
mod player;
mod websocket;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_max_level(tracing::Level::WARN)  // Only warnings and errors
        .init();

    let config = match config::Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("Robot: {}", config.robot_ip);
    println!("WebSocket: {}", config.websocket_url);

    let audio_processor = Arc::new(
        audio::AudioProcessor::new().expect("Failed to create temp dir")
    );

    let robot_player = match player::RobotPlayer::new(
        config.robot_ip.clone(),
        PathBuf::from(&config.python_script),
    ).await {
        Some(p) => Arc::new(p),
        None => {
            eprintln!("❌ Failed to initialize robot player");
            std::process::exit(1);
        }
    };

    // Larger buffer for smoother playback
    let (tx, mut rx) = mpsc::channel::<websocket::AudioMessage>(64);

    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, tx).await;
    });

    println!("🎵 Ready - waiting for audio...\n");

    // Process chunks as they arrive
    while let Some(msg) = rx.recv().await {
        if let Some(wav_path) = audio_processor.decode_and_convert(&msg.audio, &msg.format).await {
            robot_player.play_audio(&wav_path).await;
            audio_processor.cleanup(&wav_path).await;
        }
    }
}
