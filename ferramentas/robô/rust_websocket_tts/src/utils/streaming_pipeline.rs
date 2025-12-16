use tokio::sync::mpsc;
use std::sync::Arc;
use crate::utils::{audio_decoder, webrtc_audio_player, webrtc_connection};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    robot_ip: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("[PIPELINE] Audio processing pipeline started");
    println!("[PIPELINE] Waiting for first audio data from WebSocket...");
    println!("[PIPELINE]   (WebRTC connection will be established on first audio)");
    
    let mut message_count = 0;
    let mut webrtc_initialized = false;
    let mut pcm_sender: Option<mpsc::Sender<Vec<u8>>> = None;
    
    while let Some(audio_data) = audio_receiver.recv().await {
        message_count += 1;
        println!("\n[PIPELINE] ========== Message #{} ==========", message_count);
        println!("[PIPELINE] Received {} bytes of audio data from WebSocket", audio_data.len());
        
        // Initialize WebRTC on first audio message
        if !webrtc_initialized {
            println!("\n[PIPELINE] *** First audio received! Initializing WebRTC... ***");
            println!("[PIPELINE] Establishing WebRTC connection to robot at {}...", robot_ip);
            
            // Create WebRTC connection
            let webrtc_conn = Arc::new(webrtc_connection::UnitreeWebRTCConnection::new(robot_ip.clone()));
            match webrtc_conn.connect().await {
                Ok(_) => {
                    println!("[PIPELINE] ✓ WebRTC connection established");
                    
                    // Create channel for sending PCM data to audio player
                    let (sender, receiver) = mpsc::channel::<Vec<u8>>(10);
                    pcm_sender = Some(sender);
                    
                    // Create audio player with channel
                    let audio_player = webrtc_audio_player::WebRTCAudioPlayer::new(receiver);
                    let audio_track = audio_player.get_track();
                    
                    // Add audio track to WebRTC connection (like Python's conn.pc.addTrack)
                    println!("[PIPELINE] Adding audio track to WebRTC connection...");
                    match webrtc_conn.add_audio_track(audio_track).await {
                        Ok(_) => {
                            println!("[PIPELINE] ✓ Audio track added");
                            
                            // Start audio playback task
                            println!("[PIPELINE] Starting audio playback task...");
                            audio_player.start_playback().await;
                            println!("[PIPELINE] ✓ Audio player started and ready");
                            
                            webrtc_initialized = true;
                            println!("[PIPELINE] *** WebRTC initialization complete! ***\n");
                        }
                        Err(e) => {
                            eprintln!("[PIPELINE] ✗ Failed to add audio track: {}", e);
                            eprintln!("[PIPELINE] Cannot continue without audio track");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[PIPELINE] ✗ Failed to establish WebRTC connection: {}", e);
                    eprintln!("[PIPELINE] Will retry on next audio message...");
                    continue;
                }
            }
        }
        
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
                
                // Send PCM data to audio player for streaming via WebRTC
                if let Some(ref sender) = pcm_sender {
                    println!("[PIPELINE] Sending PCM to audio player for WebRTC streaming...");
                    if let Err(e) = sender.send(pcm_data).await {
                        eprintln!("[PIPELINE] ✗ Failed to send PCM to audio player: {}", e);
                    } else {
                        println!("[PIPELINE] ✓ Message #{} sent to audio player", message_count);
                    }
                } else {
                    eprintln!("[PIPELINE] ✗ Audio player not initialized yet");
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
