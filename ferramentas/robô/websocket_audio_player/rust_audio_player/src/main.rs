mod audio;
mod config;
mod player;
mod websocket;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Pre-buffer chunks before starting playback
const PRE_BUFFER_COUNT: usize = 3;

/// Maximum playback buffer size
const MAX_BUFFER: usize = 32;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let config = match config::Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    println!("🤖 Robot IP: {}", config.robot_ip);
    println!("📡 WebSocket: {}", config.websocket_url);
    println!("⚡ Workers: 4 parallel decoders\n");

    // Initialize audio processor with worker pool
    let audio_processor = Arc::new(audio::AudioProcessor::new().expect("Failed to create temp dir"));

    // Initialize robot player
    let robot_player = match player::RobotPlayer::new(
        config.robot_ip.clone(),
        PathBuf::from(&config.python_script),
    )
    .await
    {
        Some(p) => Arc::new(p),
        None => {
            eprintln!("❌ Failed to initialize robot player");
            std::process::exit(1);
        }
    };

    // Channel for WebSocket messages
    let (ws_tx, mut ws_rx) = mpsc::channel::<websocket::AudioData>(128);

    // Channel for decoded audio (separate task for decoding)
    let (decoded_tx, mut decoded_rx) = mpsc::channel::<PathBuf>(64);

    // WebSocket receiver task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    // Decoder task - converts websocket data to wav files
    let proc = Arc::clone(&audio_processor);
    tokio::spawn(async move {
        while let Some(data) = ws_rx.recv().await {
            proc.submit(&data.audio_b64, &data.format);
            
            // Continuously drain decoded results
            while let Some(path) = proc.try_recv_ordered() {
                if decoded_tx.send(path).await.is_err() {
                    return;
                }
            }
        }
    });

    // Playback buffer
    let mut playback_buffer: VecDeque<PathBuf> = VecDeque::with_capacity(MAX_BUFFER);
    let mut started = false;
    let mut chunks_played = 0u64;

    // Main playback loop
    loop {
        // Try to fill buffer from decoded chunks (non-blocking)
        while playback_buffer.len() < MAX_BUFFER {
            match decoded_rx.try_recv() {
                Ok(path) => {
                    playback_buffer.push_back(path);
                }
                Err(_) => break,
            }
        }

        // Wait for pre-buffer before starting
        if !started {
            if playback_buffer.len() >= PRE_BUFFER_COUNT {
                started = true;
                println!("🎵 Starting playback (buffered {} chunks)", playback_buffer.len());
            } else {
                // Wait for more data
                if let Some(path) = decoded_rx.recv().await {
                    playback_buffer.push_back(path);
                }
                continue;
            }
        }

        // Send ONE chunk to robot
        if let Some(path) = playback_buffer.pop_front() {
            robot_player.send_audio(&path).await;
            chunks_played += 1;

            if chunks_played % 10 == 0 {
                println!("🎵 Played: {} | Buffer: {}", chunks_played, playback_buffer.len());
            }

            // Cleanup old file in background
            let p = Arc::clone(&audio_processor);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                p.cleanup(&path);
            });
        } else {
            // Buffer empty - wait for more decoded audio
            match tokio::time::timeout(Duration::from_millis(50), decoded_rx.recv()).await {
                Ok(Some(path)) => {
                    playback_buffer.push_back(path);
                }
                Ok(None) => break, // Channel closed
                Err(_) => {} // Timeout, continue loop
            }
        }
    }
}
