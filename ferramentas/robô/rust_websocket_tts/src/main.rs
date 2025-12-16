mod utils;

use tokio::sync::mpsc;
use utils::{websocket_server, streaming_pipeline};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==============================================");
    println!("  Audio WebSocket to Unitree Go2 WebRTC");
    println!("  Direct Audio Track Streaming (like Python examples)");
    println!("==============================================");
    println!();
    
    // Get robot IP from environment or use default (localhost when running on robot)
    let robot_ip = std::env::var("ROBOT_IP")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    
    println!("Configuration:");
    println!("  Robot IP: {}", robot_ip);
    println!("  WebSocket: 0.0.0.0:8080");
    println!();
    
    // Create channel for communication between WebSocket and processing pipeline
    println!("Step 1: Creating audio processing channel...");
    let (audio_sender, audio_receiver) = mpsc::channel::<Vec<u8>>(100);
    println!("  ✓ Channel created with buffer size: 100");
    println!();
    
    // Spawn audio processing pipeline (establishes WebRTC and streams audio)
    println!("Step 2: Starting audio processing pipeline...");
    println!("  This will establish WebRTC connection and add audio track");
    let robot_ip_clone = robot_ip.clone();
    let pipeline_handle = tokio::spawn(async move {
        streaming_pipeline::start_pipeline(audio_receiver, robot_ip_clone).await
    });
    println!("  ✓ Pipeline task started");
    println!();
    
    // Start WebSocket server (listens on all interfaces)
    println!("Step 3: Starting WebSocket server...");
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
    println!("  Method: Direct WebRTC Audio Track Streaming");
    println!();
    println!("Usage:");
    println!("  Send audio data (MP3/WAV/PCM) via WebSocket");
    println!("  Audio will be decoded and streamed directly to robot");
    println!("  Similar to Python's play_mp3.py example");
    println!();
    println!("Configuration:");
    println!("  Set ROBOT_IP environment variable to change robot IP");
    println!("  Example: ROBOT_IP=192.168.123.161 cargo run");
    println!("==============================================");
    println!();
    
    // Wait for all tasks
    let result = tokio::try_join!(pipeline_handle, websocket_handle);
    
    match result {
        Ok(_) => {
            println!("All tasks completed successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("Task error: {}", e);
            Err(e.into())
        }
    }
}
