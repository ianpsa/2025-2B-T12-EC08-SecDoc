mod audio;
mod config;
mod player;
mod server;
mod websocket;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
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
    println!("🌐 HTTP Server: http://0.0.0.0:{}", config.http_port);

    // Initialize audio processor
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
    let (ws_tx, mut ws_rx) = mpsc::channel::<websocket::AudioData>(8);

    // Start HTTP server for checkpoints
    let http_player = Arc::clone(&robot_player);
    let http_processor = Arc::clone(&audio_processor);
    let audio_dir = config.audio_dir.clone();
    let http_port = config.http_port;
    
    tokio::spawn(async move {
        server::start_server(http_port, http_player, http_processor, audio_dir).await;
    });

    // WebSocket receiver task
    let ws_url = config.websocket_url.clone();
    tokio::spawn(async move {
        websocket::connect_and_receive(&ws_url, ws_tx).await;
    });

    println!("⏳ Waiting for audio...\n");

    // Simple loop: receive -> decode -> play
    while let Some(data) = ws_rx.recv().await {
        let b64_len = data.audio_b64.len();
        println!("📥 Received {} KB", b64_len / 1024);

        // Decode to WAV
        if let Some(wav_path) = audio_processor.decode(&data.audio_b64, &data.format).await {
            println!("🔄 Decoded to WAV");

            // Send to robot
            robot_player.send_audio(&wav_path).await;
            println!("🎵 Playing!\n");

            // Cleanup after 60s
            let proc = Arc::clone(&audio_processor);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                proc.cleanup(&wav_path);
            });
        } else {
            eprintln!("❌ Decode failed");
        }
    }
}
