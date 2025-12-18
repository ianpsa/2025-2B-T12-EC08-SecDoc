mod audio;
mod config;
mod player;
mod websocket;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Pre-buffer chunks before starting playback (reduces stuttering)
const PRE_BUFFER_COUNT: usize = 2;

/// Maximum playback buffer size
const MAX_BUFFER: usize = 16;

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

    // WebSocket receiver task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    // Playback buffer (ring buffer style)
    let mut playback_buffer: VecDeque<PathBuf> = VecDeque::with_capacity(MAX_BUFFER);
    let mut started = false;
    let mut chunks_received = 0u64;
    let mut chunks_played = 0u64;

    // Main processing loop - producer/consumer pattern
    let proc = Arc::clone(&audio_processor);
    
    loop {
        tokio::select! {
            // Receive from WebSocket and submit for decoding
            Some(data) = ws_rx.recv() => {
                if proc.submit(&data.audio_b64, &data.format).is_some() {
                    chunks_received += 1;
                    if chunks_received % 10 == 0 {
                        println!("📥 Received: {} | Played: {} | Buffer: {}", 
                            chunks_received, chunks_played, playback_buffer.len());
                    }
                }
            }
            
            // Poll for decoded chunks (non-blocking)
            _ = tokio::time::sleep(Duration::from_millis(5)) => {
                // Collect decoded chunks into buffer
                while let Some(path) = proc.try_recv_ordered() {
                    if playback_buffer.len() < MAX_BUFFER {
                        playback_buffer.push_back(path);
                    } else {
                        // Buffer full, cleanup oldest
                        if let Some(old) = playback_buffer.pop_front() {
                            proc.cleanup(&old);
                        }
                        playback_buffer.push_back(path);
                    }
                }

                // Start playback after pre-buffer
                if !started && playback_buffer.len() >= PRE_BUFFER_COUNT {
                    started = true;
                    println!("🎵 Starting playback (buffered {} chunks)", playback_buffer.len());
                }

                // Send chunks to robot
                if started {
                    while let Some(path) = playback_buffer.pop_front() {
                        robot_player.send_audio(&path).await;
                        chunks_played += 1;

                        // Cleanup in background
                        let p = proc.clone();
                        let path_clone = path.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                            p.cleanup(&path_clone);
                        });
                    }
                }
            }
        }
    }
}
