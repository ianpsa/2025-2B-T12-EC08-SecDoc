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
            println!("[PIPELINE] Setting up WebRTC connection to robot at {}...", robot_ip);
            
            // Create WebRTC connection
            let webrtc_conn = Arc::new(webrtc_connection::UnitreeWebRTCConnection::new(robot_ip.clone()));
            
            // Step 1: Setup peer connection (like Python: pc = RTCPeerConnection())
            match webrtc_conn.setup().await {
                Ok(_) => {
                    println!("[PIPELINE] ✓ Peer connection setup complete");
                    
                    // Step 2: Create audio player and get track
                    // Create channel for sending PCM data to audio player
                    let (sender, receiver) = mpsc::channel::<Vec<u8>>(10);
                    pcm_sender = Some(sender);
                    
                    let audio_player = webrtc_audio_player::WebRTCAudioPlayer::new(receiver);
                    let audio_track = audio_player.get_track();
                    
                    // Step 3: Add audio track BEFORE creating offer (like Python: conn.pc.addTrack(audio_track))
                    println!("[PIPELINE] Adding audio track to peer connection...");
                    match webrtc_conn.add_audio_track(audio_track).await {
                        Ok(_) => {
                            println!("[PIPELINE] ✓ Audio track added");
                            
                            // Step 4: NOW establish WebRTC connection (creates offer with audio track)
                            println!("[PIPELINE] Establishing WebRTC connection (creating offer + signaling)...");
                            match webrtc_conn.connect().await {
                                Ok(_) => {
                                    println!("[PIPELINE] ✓ Signaling complete");
                                    
                                    // Step 5: Wait for connection to be fully established
                                    println!("[PIPELINE] Waiting for WebRTC connection to complete (ICE + DTLS handshake)...");
                                    match webrtc_conn.wait_until_connected().await {
                                        Ok(_) => {
                                            println!("[PIPELINE] ✓ WebRTC connection fully established");
                                            
                                            // Step 6: NOW start audio playback (connection is ready)
                                            println!("[PIPELINE] Starting audio playback task...");
                                            audio_player.start_playback().await;
                                            println!("[PIPELINE] ✓ Audio player started");
                                            
                                            webrtc_initialized = true;
                                            println!("[PIPELINE] *** WebRTC initialization complete! ***\n");
                                        }
                                        Err(e) => {
                                            eprintln!("[PIPELINE] ✗ Timeout waiting for connection: {}", e);
                                            eprintln!("[PIPELINE] The signaling completed but ICE/DTLS handshake failed");
                                            eprintln!("[PIPELINE] Will retry on next audio message...");
                                            continue;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[PIPELINE] ✗ Failed to establish WebRTC connection: {}", e);
                                    eprintln!("[PIPELINE]");
                                    eprintln!("[PIPELINE] Common issues:");
                                    eprintln!("[PIPELINE]   1. Robot IP: Current ROBOT_IP={}", robot_ip);
                                    if robot_ip == "127.0.0.1" {
                                        eprintln!("[PIPELINE]      ✓ Using localhost (correct for running ON robot)");
                                        eprintln!("[PIPELINE]      - Is this service running ON the robot itself?");
                                        eprintln!("[PIPELINE]      - Check if robot's WebRTC service is running:");
                                        eprintln!("[PIPELINE]        curl http://127.0.0.1:8081/");
                                    } else {
                                        eprintln!("[PIPELINE]      ⚠ Using external IP (only works if running on laptop)");
                                        eprintln!("[PIPELINE]      - This service should run ON the robot!");
                                        eprintln!("[PIPELINE]      - See DEPLOY_ON_ROBOT.md for instructions");
                                        eprintln!("[PIPELINE]      - If running on robot, use: ROBOT_IP=127.0.0.1");
                                    }
                                    eprintln!("[PIPELINE]");
                                    eprintln!("[PIPELINE]   2. Robot WebRTC service not running:");
                                    eprintln!("[PIPELINE]      - The robot's built-in WebRTC server must be active");
                                    eprintln!("[PIPELINE]      - Port 8081 must be listening on localhost");
                                    eprintln!("[PIPELINE]");
                                    eprintln!("[PIPELINE]   3. Architecture:");
                                    eprintln!("[PIPELINE]      - This service MUST run ON the robot (not laptop)");
                                    eprintln!("[PIPELINE]      - Robot's WebRTC only accessible from localhost");
                                    eprintln!("[PIPELINE]      - Send audio from laptop to robot via WebSocket");
                                    eprintln!("[PIPELINE]");
                                    eprintln!("[PIPELINE] Will retry on next audio message...");
                                    eprintln!("[PIPELINE]");
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[PIPELINE] ✗ Failed to add audio track: {}", e);
                            eprintln!("[PIPELINE] Cannot continue without audio track");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[PIPELINE] ✗ Failed to setup peer connection: {}", e);
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
