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

    // Stage 1: WebSocket -> Raw audio messages
    let (ws_tx, ws_rx) = mpsc::channel::<websocket::AudioMessage>(64);
    
    // Stage 2: Decoded WAV files (ordered)
    let (wav_tx, wav_rx) = mpsc::channel::<PathBuf>(16);

    // WebSocket receiver task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    // Decoder task - decodes ahead while previous chunk plays
    let processor = Arc::clone(&audio_processor);
    tokio::spawn(async move {
        decode_pipeline(ws_rx, wav_tx, processor).await;
    });

    // Player task - plays from decoded queue
    play_pipeline(robot_player, wav_rx, audio_processor).await;
}

/// Decode chunks sequentially but don't wait for playback
async fn decode_pipeline(
    mut rx: mpsc::Receiver<websocket::AudioMessage>,
    tx: mpsc::Sender<PathBuf>,
    processor: Arc<audio::AudioProcessor>,
) {
    while let Some(msg) = rx.recv().await {
        if let Some(wav_path) = processor.decode_and_convert(&msg.audio, &msg.format).await {
            if tx.send(wav_path).await.is_err() {
                break;
            }
        }
    }
}

/// Play chunks from queue - Python handles timing
async fn play_pipeline(
    player: Arc<player::RobotPlayer>,
    mut rx: mpsc::Receiver<PathBuf>,
    processor: Arc<audio::AudioProcessor>,
) {
    while let Some(wav_path) = rx.recv().await {
        player.send_audio(&wav_path).await;
        
        // Cleanup in background
        let proc = Arc::clone(&processor);
        let path = wav_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            proc.cleanup(&path).await;
        });
    }
}
