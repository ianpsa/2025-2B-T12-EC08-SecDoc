mod utils;

use std::sync::Arc;
use tokio::sync::mpsc;
use utils::{webrtc_audio_hub, websocket_server, streaming_pipeline};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==============================================");
    println!("  Audio WebSocket to Unitree Go2 WebRTC");
    println!("==============================================");
    println!();
    
    // Get robot IP from environment or use default
    let robot_ip = std::env::var("ROBOT_IP")
        .unwrap_or_else(|_| "192.168.8.181".to_string());
    
    println!("Step 1: Initializing WebRTC Audio Hub...");
    println!("  Robot IP: {}", robot_ip);
    let audio_hub = Arc::new(webrtc_audio_hub::WebRTCAudioHub::new(robot_ip.clone()));
    println!("  ✓ WebRTC Audio Hub created");
    println!();
    
    println!("Step 2: Connecting to robot via WebRTC...");
    audio_hub.connect().await?;
    println!("  ✓ WebRTC connection established");
    println!();
    
    // Create channel for communication between WebSocket and processing pipeline
    println!("Step 3: Creating audio processing channel...");
    let (audio_sender, audio_receiver) = mpsc::channel::<Vec<u8>>(100);
    println!("  ✓ Channel created with buffer size: 100");
    println!();
    
    // Spawn audio processing pipeline
    println!("Step 4: Starting audio processing pipeline...");
    let audio_hub_clone = Arc::clone(&audio_hub);
    let pipeline_handle = tokio::spawn(async move {
        streaming_pipeline::start_pipeline(audio_receiver, audio_hub_clone).await
    });
    println!("  ✓ Pipeline task started");
    println!();
    
    // Start WebSocket server (listens on all interfaces)
    println!("Step 5: Starting WebSocket server...");
    let websocket_handle = tokio::spawn(async move {
        websocket_server::start_websocket_server("0.0.0.0:8080", audio_sender).await
    });
    println!("  ✓ WebSocket server task started on 0.0.0.0:8080");
    println!();
    
    println!("==============================================");
    println!("  Service is ready!");
    println!("==============================================");
    println!("  WebSocket: ws://0.0.0.0:8080");
    println!("  Robot IP: {}", robot_ip);
    println!("  Audio API: WebRTC Data Channel");
    println!();
    println!("Configuration:");
    println!("  Set ROBOT_IP environment variable to change robot IP");
    println!("  Example: ROBOT_IP=192.168.8.181 cargo run");
    println!("==============================================");
    println!();
    
    // Wait for all tasks
    tokio::try_join!(pipeline_handle, websocket_handle).map(|_| ())?;
    
    Ok(())
}
