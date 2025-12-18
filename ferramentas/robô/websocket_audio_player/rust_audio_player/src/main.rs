mod audio;
mod config;
mod player;
mod websocket;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

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

    // WebSocket -> messages
    let (ws_tx, mut ws_rx) = mpsc::channel::<websocket::AudioMessage>(64);
    
    // Decoded results (ordered by sequence number)
    let decoded_buffer: Arc<Mutex<BTreeMap<u64, PathBuf>>> = Arc::new(Mutex::new(BTreeMap::new()));
    let next_to_play: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    
    // Ready to play channel
    let (play_tx, mut play_rx) = mpsc::channel::<PathBuf>(32);

    // WebSocket task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    // Parallel decode dispatcher
    let processor = Arc::clone(&audio_processor);
    let buffer = Arc::clone(&decoded_buffer);
    let next_seq = Arc::clone(&next_to_play);
    let sender = play_tx.clone();
    
    tokio::spawn(async move {
        let mut seq: u64 = 0;
        
        while let Some(msg) = ws_rx.recv().await {
            let current_seq = seq;
            seq += 1;
            
            let proc = Arc::clone(&processor);
            let buf = Arc::clone(&buffer);
            let next = Arc::clone(&next_seq);
            let tx = sender.clone();
            
            // Spawn parallel decode task
            tokio::spawn(async move {
                if let Some(wav_path) = proc.decode_and_convert(&msg.audio, &msg.format).await {
                    // Store result
                    {
                        let mut map = buf.lock().await;
                        map.insert(current_seq, wav_path);
                    }
                    
                    // Check if we can send next in order
                    loop {
                        let mut next_guard = next.lock().await;
                        let mut map = buf.lock().await;
                        
                        if let Some(path) = map.remove(&*next_guard) {
                            *next_guard += 1;
                            drop(map);
                            drop(next_guard);
                            let _ = tx.send(path).await;
                        } else {
                            break;
                        }
                    }
                }
            });
        }
    });

    // Player task
    let proc = Arc::clone(&audio_processor);
    while let Some(wav_path) = play_rx.recv().await {
        robot_player.send_audio(&wav_path).await;
        
        // Background cleanup
        let p = Arc::clone(&proc);
        let path = wav_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            p.cleanup(&path).await;
        });
    }
}
