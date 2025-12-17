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
        .init();

    let config = match config::Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    info!("🤖 Robot IP: {}", config.robot_ip);
    info!("🌐 WebSocket: {}", config.websocket_url);

    let audio_processor = Arc::new(audio::AudioProcessor::new().expect("Failed to create temp dir"));
    let robot_player = Arc::new(player::RobotPlayer::new(
        config.robot_ip.clone(),
        PathBuf::from(&config.python_script),
    ));

    let (tx, mut rx) = mpsc::channel::<websocket::AudioMessage>(32);

    // WebSocket receiver task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, tx).await;
    });

    // Audio processing loop
    while let Some(msg) = rx.recv().await {
        let processor = Arc::clone(&audio_processor);
        let player = Arc::clone(&robot_player);

        tokio::spawn(async move {
            if let Some(wav_path) = processor.decode_and_convert(&msg.audio, &msg.format).await {
                player.play_audio(&wav_path).await;
                processor.cleanup(&wav_path).await;
            }
        });
    }
}

