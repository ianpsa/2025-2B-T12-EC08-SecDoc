mod utils;

use tokio::sync::mpsc;
use utils::{websocket_server, streaming_pipeline};
use std::env;

/// Main entry point for WebSocket to WebRTC Audio Streaming Service
/// 
/// This application receives audio data via WebSocket and streams it to the
/// Unitree Go2 robot via WebRTC, similar to the Python examples in go2_webrtc.
/// 
/// Architecture:
/// 1. WebSocket Server: Receives audio data (MP3/WAV/PCM) from clients
/// 2. Audio Pipeline: Decodes and processes audio into PCM format
/// 3. WebRTC Connection: Streams PCM audio directly to the robot
/// 
/// Based on: ferramentas/robô/rust_websocket_tts/go2_webrtc/examples/go2/audio/mp3_player/play_mp3.py
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    
    // Configuration
    let config = load_configuration();
    print_configuration(&config);
    
    // Step 1: Create communication channel
    println!("Step 1: Creating audio processing channel...");
    let (audio_sender, audio_receiver) = mpsc::channel::<Vec<u8>>(100);
    println!("  ✓ Channel created with buffer size: 100");
    println!();
    
    // Step 2: Start audio processing pipeline (lazy WebRTC connection)
    println!("Step 2: Starting audio processing pipeline...");
    println!("  WebRTC connection will be established when first audio is received");
    let robot_ip_clone = config.robot_ip.clone();
    let pipeline_handle = tokio::spawn(async move {
        streaming_pipeline::start_pipeline(audio_receiver, robot_ip_clone).await
    });
    println!("  ✓ Pipeline task started and waiting");
    println!();
    
    // Step 3: Start WebSocket server
    println!("Step 3: Starting WebSocket server...");
    let websocket_handle = tokio::spawn(async move {
        websocket_server::start_websocket_server(&config.websocket_addr, audio_sender).await
    });
    println!("  ✓ WebSocket server task started on {}", config.websocket_addr);
    println!();
    
    print_service_ready(&config);
    
    // Wait for all tasks to complete
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

/// Configuration structure for the service
struct Config {
    robot_ip: String,
    websocket_addr: String,
}

/// Load configuration from environment variables or use defaults
fn load_configuration() -> Config {
    Config {
        robot_ip: env::var("ROBOT_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
        websocket_addr: env::var("WEBSOCKET_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
    }
}

/// Print the welcome banner
fn print_banner() {
    println!("==============================================");
    println!("  Audio WebSocket to Unitree Go2 WebRTC");
    println!("  Direct Audio Track Streaming");
    println!("==============================================");
    println!();
    println!("Based on: go2_webrtc/examples/go2/audio/");
    println!("  - mp3_player/play_mp3.py");
    println!("  - mp3_player/webrtc_audio_player.py");
    println!();
}

/// Print the current configuration
fn print_configuration(config: &Config) {
    println!("Configuration:");
    println!("  Robot IP: {}", config.robot_ip);
    println!("  WebSocket: {}", config.websocket_addr);
    println!("  Connection Mode: Lazy (connects on first audio)");
    println!();
}

/// Print service ready message
fn print_service_ready(config: &Config) {
    println!("==============================================");
    println!("  Service is ready!");
    println!("==============================================");
    println!("  WebSocket: ws://{}", config.websocket_addr);
    println!("  Robot IP: {}", config.robot_ip);
    println!("  Method: Direct WebRTC Audio Track Streaming");
    println!();
    println!("Status:");
    println!("  ⏳ Waiting for first audio via WebSocket...");
    println!("  ⏳ WebRTC will connect automatically on first audio");
    println!();
    println!("Usage:");
    println!("  Send audio data (MP3/WAV/PCM) via WebSocket");
    println!("  Audio will be decoded and streamed directly to robot");
    println!("  Similar to Python's play_mp3.py example");
    println!();
    println!("Client Example:");
    println!("  python3 audio_stream.py --file audio.mp3");
    println!();
    println!("Configuration:");
    println!("  Set ROBOT_IP environment variable to change robot IP");
    println!("  Default: 127.0.0.1 (localhost - for running ON robot)");
    println!("  Example: ROBOT_IP=127.0.0.1 cargo run");
    println!();
    println!("  Set WEBSOCKET_ADDR to change WebSocket bind address");
    println!("  Default: 0.0.0.0:8080");
    println!("  Example: WEBSOCKET_ADDR=0.0.0.0:9090 cargo run");
    println!();
    println!("⚠️  IMPORTANT: This service must run ON the robot!");
    println!("   See DEPLOY_ON_ROBOT.md for deployment instructions");
    println!("==============================================");
    println!();
}
