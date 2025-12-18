mod audio;
mod config;
mod player;
mod websocket;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

const PRE_BUFFER_COUNT: usize = 3; // Wait for 3 chunks before starting playback

#[tokio::main]
async fn main() {
    let config = match config::Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("Robot: {}", config.robot_ip);
    println!("WebSocket: {}", config.websocket_url);

    let audio_processor = Arc::new(audio::AudioProcessor::new().expect("temp dir"));

    let robot_player = match player::RobotPlayer::new(
        config.robot_ip.clone(),
        PathBuf::from(&config.python_script),
    ).await {
        Some(p) => Arc::new(p),
        None => {
            eprintln!("Failed to init player");
            std::process::exit(1);
        }
    };

    let (ws_tx, mut ws_rx) = mpsc::channel::<websocket::AudioMessage>(64);
    let (decoded_tx, mut decoded_rx) = mpsc::channel::<PathBuf>(32);

    // WebSocket receiver
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    // Parallel decoder - spawns multiple FFmpeg
    let proc = Arc::clone(&audio_processor);
    let dtx = decoded_tx.clone();
    tokio::spawn(async move {
        let mut handles = Vec::new();
        let mut seq = 0u64;
        
        while let Some(msg) = ws_rx.recv().await {
            let p = Arc::clone(&proc);
            let tx = dtx.clone();
            let s = seq;
            seq += 1;
            
            let h = tokio::spawn(async move {
                if let Some(path) = p.decode_and_convert(&msg.audio, &msg.format).await {
                    (s, path)
                } else {
                    (s, PathBuf::new())
                }
            });
            handles.push(h);
            
            // Process completed decodes
            let mut i = 0;
            while i < handles.len() {
                if handles[i].is_finished() {
                    if let Ok((_, path)) = handles.remove(i).await {
                        if path.exists() {
                            let _ = tx.send(path).await;
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }
        
        // Finish remaining
        for h in handles {
            if let Ok((_, path)) = h.await {
                if path.exists() {
                    let _ = dtx.send(path).await;
                }
            }
        }
    });

    // Pre-buffer then play continuously
    let mut buffer: Vec<PathBuf> = Vec::new();
    let mut started = false;
    
    while let Some(path) = decoded_rx.recv().await {
        buffer.push(path);
        
        // Wait for pre-buffer before starting
        if !started && buffer.len() >= PRE_BUFFER_COUNT {
            started = true;
            println!("Buffered {} chunks, starting playback", PRE_BUFFER_COUNT);
        }
        
        // Send buffered chunks
        if started {
            while let Some(p) = buffer.first().cloned() {
                buffer.remove(0);
                robot_player.send_audio(&p).await;
                
                // Cleanup later
                let proc = Arc::clone(&audio_processor);
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    proc.cleanup(&p).await;
                });
            }
        }
    }
    
    // Flush remaining
    for p in buffer {
        robot_player.send_audio(&p).await;
    }
}
