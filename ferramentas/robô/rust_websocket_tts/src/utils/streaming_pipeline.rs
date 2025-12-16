use tokio::sync::mpsc;
use std::sync::Arc;
use crate::utils::{audio_decoder, webrtc_audio_player, webrtc_connection};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    robot_ip: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("[PIPELINE] Audio processing pipeline started");
    println!("[PIPELINE] Establishing WebRTC connection to robot...");
    
    // Create WebRTC connection
    let webrtc_conn = Arc::new(webrtc_connection::UnitreeWebRTCConnection::new(robot_ip));
    webrtc_conn.connect().await?;
    println!("[PIPELINE] ✓ WebRTC connection established");
    
    // Create channel for sending PCM data to audio player
    let (pcm_sender, pcm_receiver) = mpsc::channel::<Vec<u8>>(10);
    
    // Create audio player with channel
    let audio_player = webrtc_audio_player::WebRTCAudioPlayer::new(pcm_receiver);
    let audio_track = audio_player.get_track();
    
    // Add audio track to WebRTC connection (like Python's conn.pc.addTrack)
    println!("[PIPELINE] Adding audio track to WebRTC connection...");
    webrtc_conn.add_audio_track(audio_track).await?;
    println!("[PIPELINE] ✓ Audio track added");
    
    // Start audio playback task
    println!("[PIPELINE] Starting audio playback task...");
    audio_player.start_playback().await;
    println!("[PIPELINE] ✓ Audio player started");
    
    println!("[PIPELINE] Waiting for audio data from WebSocket...");
    
    let mut message_count = 0;
    
    while let Some(audio_data) = audio_receiver.recv().await {
        message_count += 1;
        println!("\n[PIPELINE] ========== Message #{} ==========", message_count);
        println!("[PIPELINE] Received {} bytes of audio data from WebSocket", audio_data.len());
        
        // Show first few bytes for debugging
        let preview_len = 16.min(audio_data.len());
        println!("[PIPELINE] First {} bytes: {:02X?}", preview_len, &audio_data[..preview_len]);
        
        // Process audio (handles both encoded and raw PCM)
        println!("[PIPELINE] Processing audio data...");
        match audio_decoder::process_audio(audio_data) {
            Ok(pcm_data) => {
                println!("[PIPELINE] ✓ Audio processing successful");
                println!("[PIPELINE]   - Output PCM size: {} bytes", pcm_data.len());
                println!("[PIPELINE]   - Audio duration (approx): {:.2}s @ 16kHz mono", 
                    pcm_data.len() as f64 / (16000.0 * 2.0)); // 2 bytes per sample
                
                // Show first few bytes of PCM
                let pcm_preview_len = 16.min(pcm_data.len());
                println!("[PIPELINE]   - First {} PCM bytes: {:02X?}", pcm_preview_len, &pcm_data[..pcm_preview_len]);
                
                // Send PCM data directly to audio player for streaming via WebRTC
                println!("[PIPELINE] Sending PCM to audio player for WebRTC streaming...");
                if let Err(e) = pcm_sender.send(pcm_data).await {
                    eprintln!("[PIPELINE] ✗ Failed to send PCM to audio player: {}", e);
                } else {
                    println!("[PIPELINE] ✓ Message #{} sent to audio player", message_count);
                }
            }
            Err(e) => {
                eprintln!("[PIPELINE] ✗ Failed to process audio for message #{}: {}", message_count, e);
            }
        }
        println!("[PIPELINE] ========================================\n");
    }
    
    println!("[PIPELINE] Audio receiver channel closed, pipeline shutting down");
    Ok(())
}
