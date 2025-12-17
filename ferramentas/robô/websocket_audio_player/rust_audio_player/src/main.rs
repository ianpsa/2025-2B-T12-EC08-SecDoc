mod audio;
mod config;
mod player;
mod websocket;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .init();

    let config = match config::Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    info!("Robot IP: {}", config.robot_ip);
    info!("WebSocket: {}", config.websocket_url);

    let audio_processor = Arc::new(
        audio::AudioProcessor::new().expect("Failed to create temp dir")
    );

    let robot_player = match player::RobotPlayer::new(
        config.robot_ip.clone(),
        PathBuf::from(&config.python_script),
    ).await {
        Some(p) => Arc::new(p),
        None => {
            error!("Failed to initialize robot player");
            std::process::exit(1);
        }
    };

    let (tx, mut rx) = mpsc::channel::<websocket::AudioMessage>(32);

    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, tx).await;
    });

    info!("Waiting for audio...");

    while let Some(msg) = rx.recv().await {
        let processor = Arc::clone(&audio_processor);
        let player = Arc::clone(&robot_player);

        tokio::spawn(async move {
            if let Some(wav_path) = processor.decode_and_convert(&msg.audio, &msg.format).await {
                if wav_path.exists() {
                    player.play_audio(&wav_path).await;
                    processor.cleanup(&wav_path).await;
                } else {
                    error!("WAV file missing: {:?}", wav_path);
                }
            }
        });
    }
}
