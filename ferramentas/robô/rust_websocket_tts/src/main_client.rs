mod utils;

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==============================================");
    println!("  Unitree Go2 Audio Client");
    println!("  WebSocket Client → Local Audio Playback");
    println!("==============================================");
    println!();
    
    // Get server URL from environment or use default
    let server_url = std::env::var("SERVER_URL")
        .unwrap_or_else(|_| "ws://192.168.123.1:8080".to_string());
    
    println!("Configuration:");
    println!("  Server URL: {}", server_url);
    println!("  Mode: WebSocket Client (runs ON robot)");
    println!();
    
    println!("Connecting to server...");
    
    let url = Url::parse(&server_url)?;
    let (ws_stream, _) = connect_async(url).await?;
    
    println!("✓ Connected to server!");
    println!("Waiting for audio data...");
    println!();
    
    let (write, mut read) = ws_stream.split();
    
    let mut message_count = 0;
    
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(audio_data)) => {
                message_count += 1;
                println!("[AUDIO] Received message #{}: {} bytes", message_count, audio_data.len());
                
                // Decode audio using FFmpeg
                match utils::audio_decoder::process_audio(audio_data) {
                    Ok(pcm_data) => {
                        println!("[AUDIO] ✓ Decoded to {} bytes PCM", pcm_data.len());
                        
                        // Play audio locally using ALSA/PulseAudio
                        // TODO: Implement local audio playback
                        println!("[AUDIO] TODO: Play audio via ALSA/PulseAudio");
                    }
                    Err(e) => {
                        eprintln!("[AUDIO] ✗ Decode failed: {}", e);
                    }
                }
            }
            Ok(Message::Text(text)) => {
                println!("[SERVER] Message: {}", text);
            }
            Ok(Message::Close(_)) => {
                println!("Server closed connection");
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    println!("Connection closed");
    Ok(())
}
