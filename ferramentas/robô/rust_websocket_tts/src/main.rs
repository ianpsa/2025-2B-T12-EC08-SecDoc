mod mp3_decoder;
mod utils {
    pub mod websocket_handler;
    pub mod ros_audio_publisher;
}

use anyhow::Result;
use tracing::{info, error, Level};
use tracing_subscriber;
use tokio::signal;

use crate::utils::websocket_handler::WebSocketAudioClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("=== Rust WebSocket → ROS2 Audio Bridge ===");
    info!("🤖 Unitree GO2 Audio Client");
    info!("Starting initialization...");

    // Configuration
    let ws_url = std::env::var("WS_URL")
        .unwrap_or_else(|_| "ws://localhost:8080/v1/audio".to_string());

    info!("Configuration:");
    info!("  WebSocket URL: {}", ws_url);
    info!("  ROS2 Topic: audiodata");

    // Create WebSocket client (this also initializes ROS2)
    info!("Creating WebSocket audio client with ROS2 publisher...");
    let ws_client = match WebSocketAudioClient::new(ws_url.clone()) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create WebSocket client: {}", e);
            error!("Make sure ROS2 is installed and configured");
            error!("Run: source /opt/ros/<distro>/setup.bash");
            return Err(e);
        }
    };
    info!("WebSocket client created successfully");

    info!("=== Client Ready ===");
    info!("Connecting to: {}", ws_url);
    info!("");
    info!("Pipeline:");
    info!("  WebSocket → Base64 decode (if needed) → MP3 decode → ROS2 publish → Robot plays");
    info!("");
    info!("Message flow:");
    info!("  → Send: {{\"type\": \"text\", \"texto\": \"...\", ...}}");
    info!("  ← Receive: Text response (JSON)");
    info!("  ← Receive: Binary audio (MP3) or Base64 audio");
    info!("  → Decode MP3 to PCM");
    info!("  → Publish to ROS2 topic 'audiodata'");
    info!("  ← Receive: {{\"done\": true}}");
    info!("");
    info!("Press Ctrl+C to exit gracefully");
    info!("");

    // Run the WebSocket client (will auto-reconnect on disconnect)
    loop {
        tokio::select! {
            result = ws_client.connect_and_listen() => {
                match result {
                    Ok(_) => {
                        info!("Connection closed normally");
                    }
                    Err(e) => {
                        error!("Connection error: {}", e);
                        error!("Retrying in 5 seconds...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
            _ = signal::ctrl_c() => {
                info!("");
                info!("Received shutdown signal (Ctrl+C)");
                info!("Shutting down gracefully...");
                break;
            }
        }
    }

    info!("✓ Shutdown complete");
    Ok(())
}
