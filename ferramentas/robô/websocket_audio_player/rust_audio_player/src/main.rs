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

    // Process audio messages with streaming chunks
    while let Some(msg) = rx.recv().await {
        info!("Received audio - starting streaming playback");
        
        // Decode base64 to file
        let input_path = match audio_processor.decode_to_file(&msg.audio, &msg.format).await {
            Some(p) => p,
            None => {
                error!("Failed to decode audio");
                continue;
            }
        };

        // Start megaphone mode for streaming
        robot_player.start_streaming().await;

        // Convert to chunks and stream them as they're ready
        let mut chunk_rx = audio_processor.convert_to_streaming_chunks(&input_path).await;
        let mut chunk_count = 0;

        while let Some(chunk_path) = chunk_rx.recv().await {
            chunk_count += 1;
            // Stream each chunk as it becomes available
            robot_player.stream_chunk(&chunk_path).await;
            audio_processor.cleanup(&chunk_path).await;
        }

        // End streaming
        robot_player.stop_streaming().await;
        info!("Streamed {} chunks", chunk_count);
    }
}
