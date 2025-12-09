// Main entry point for the Rust WebSocket Audio Client
// This application connects to a WebSocket server and plays received audio

mod audio_decoder;
mod utils {
    pub mod websocket_handler;
}

use anyhow::Result;
use tracing::{info, error, Level};
use tracing_subscriber;

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

    info!("=== Rust WebSocket Audio Client ===");
    info!("Starting initialization...");

    // Configuration
    let ws_url = std::env::var("WS_URL")
        .unwrap_or_else(|_| "ws://localhost:8080/v1/audio".to_string());

    info!("Configuration:");
    info!("  WebSocket URL: {}", ws_url);

    // Create WebSocket client
    info!("Creating WebSocket audio client...");
    let ws_client = match WebSocketAudioClient::new(ws_url.clone()) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create WebSocket client: {}", e);
            error!("Make sure audio devices are available");
            return Err(e);
        }
    };
    info!("WebSocket client created successfully");

    info!("=== Client Ready ===");
    info!("Connecting to: {}", ws_url);
    info!("");
    info!("Usage:");
    info!("  This client connects to the backend WebSocket and can:");
    info!("  1. Send text questions to the backend");
    info!("  2. Receive text response + audio response");
    info!("");
    info!("Message flow:");
    info!("  → Send: {{\"type\": \"text\", \"texto\": \"...\", ...}}");
    info!("  ← Receive: Text response (JSON)");
    info!("  ← Receive: Binary audio (MP3/WAV/OGG)");
    info!("  ← Receive: {{\"done\": true}}");
    info!("");

    // Run the WebSocket client (will auto-reconnect on disconnect)
    loop {
        match ws_client.connect_and_listen().await {
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
}
