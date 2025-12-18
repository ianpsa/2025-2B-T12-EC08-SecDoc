mod audio;
mod config;
mod player;
mod websocket;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_max_level(tracing::Level::ERROR)
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
            eprintln!("Failed to initialize robot player");
            std::process::exit(1);
        }
    };

    // Channel for receiving audio from websocket
    let (ws_tx, mut ws_rx) = mpsc::channel::<websocket::AudioMessage>(128);
    
    // Channel for sending processed WAV paths to player
    let (wav_tx, wav_rx) = mpsc::channel::<PathBuf>(32);

    // WebSocket receiver task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    // Audio processing task - converts chunks in background
    let processor = Arc::clone(&audio_processor);
    let wav_sender = wav_tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = ws_rx.recv().await {
            if let Some(wav_path) = processor.decode_and_convert(&msg.audio, &msg.format).await {
                if wav_sender.send(wav_path).await.is_err() {
                    break;
                }
            }
        }
    });


    // Player task - plays chunks as they become ready (no waiting between)
    player::play_continuous(robot_player, wav_rx, audio_processor).await;
}
