use tokio::sync::mpsc;
use std::sync::Arc;
use crate::utils::{audio_decoder, webrtc_audio_hub};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    audio_hub: Arc<webrtc_audio_hub::WebRTCAudioHub>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("[PIPELINE] Audio processing pipeline started");
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
                
                // Convert PCM to WAV format for Go2 (44.1kHz mono)
                println!("[PIPELINE] Converting to WAV format for Go2...");
                match convert_pcm_to_wav(&pcm_data).await {
                    Ok(wav_data) => {
                        println!("[PIPELINE] ✓ WAV conversion successful: {} bytes", wav_data.len());
                        
                        // Generate unique filename
                        let file_name = format!("audio_{}", message_count);
                        
                        // Upload to robot via WebRTC
                        println!("[PIPELINE] Uploading to robot via WebRTC...");
                        match audio_hub.upload_audio_wav(&file_name, wav_data).await {
                            Ok(file_md5) => {
                                println!("[PIPELINE] ✓ Audio uploaded successfully");
                                println!("[PIPELINE]   - File: {}", file_name);
                                println!("[PIPELINE]   - MD5: {}", file_md5);
                                
                                // Small delay before playback
                                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                
                                // Get audio list to find UUID
                                println!("[PIPELINE] Fetching audio list to get UUID...");
                                match audio_hub.get_audio_list().await {
                                    Ok(audio_list) => {
                                        // Find the uploaded audio by name
                                        if let Some(audio_info) = audio_list.data.iter()
                                            .find(|a| a.file_name == file_name) {
                                            
                                            println!("[PIPELINE] Found audio UUID: {}", audio_info.unique_id);
                                            
                                            // Trigger playback
                                            println!("[PIPELINE] Triggering playback...");
                                            match audio_hub.play_by_uuid(&audio_info.unique_id).await {
                                                Ok(_) => {
                                                    println!("[PIPELINE] ✓ Message #{} fully processed and playing", message_count);
                                                }
                                                Err(e) => {
                                                    eprintln!("[PIPELINE] ✗ Failed to play audio: {}", e);
                                                }
                                            }
                                        } else {
                                            eprintln!("[PIPELINE] ✗ Could not find uploaded audio in list");
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("[PIPELINE] ✗ Failed to get audio list: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[PIPELINE] ✗ Failed to upload audio for message #{}: {}", message_count, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[PIPELINE] ✗ Failed to convert to WAV: {}", e);
                    }
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

/// Convert raw PCM data to WAV format (44.1kHz mono) for Go2 compatibility
async fn convert_pcm_to_wav(pcm_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Write;

    // Assume input is 16kHz, 16-bit mono PCM (from audio_decoder)
    let sample_rate_in = 16000u32;
    let sample_rate_out = 44100u32;
    let bits_per_sample = 16u16;
    let num_channels = 1u16;
    
    // Calculate number of samples
    let num_samples_in = pcm_data.len() / 2; // 16-bit = 2 bytes per sample
    
    // Resample from 16kHz to 44.1kHz
    let ratio = sample_rate_out as f64 / sample_rate_in as f64;
    let num_samples_out = (num_samples_in as f64 * ratio) as usize;
    
    let mut resampled = Vec::with_capacity(num_samples_out * 2);
    
    for i in 0..num_samples_out {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos as usize;
        
        if src_idx < num_samples_in {
            let byte_idx = src_idx * 2;
            if byte_idx + 1 < pcm_data.len() {
                resampled.push(pcm_data[byte_idx]);
                resampled.push(pcm_data[byte_idx + 1]);
            }
        }
    }
    
    // Build WAV header
    let data_size = resampled.len() as u32;
    let byte_rate = sample_rate_out * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    
    let mut wav_data = Vec::new();
    
    // RIFF header
    wav_data.write_all(b"RIFF")?;
    wav_data.write_all(&(36 + data_size).to_le_bytes())?;
    wav_data.write_all(b"WAVE")?;
    
    // fmt chunk
    wav_data.write_all(b"fmt ")?;
    wav_data.write_all(&16u32.to_le_bytes())?; // fmt chunk size
    wav_data.write_all(&1u16.to_le_bytes())?;  // audio format (1 = PCM)
    wav_data.write_all(&num_channels.to_le_bytes())?;
    wav_data.write_all(&sample_rate_out.to_le_bytes())?;
    wav_data.write_all(&byte_rate.to_le_bytes())?;
    wav_data.write_all(&block_align.to_le_bytes())?;
    wav_data.write_all(&bits_per_sample.to_le_bytes())?;
    
    // data chunk
    wav_data.write_all(b"data")?;
    wav_data.write_all(&data_size.to_le_bytes())?;
    wav_data.write_all(&resampled)?;
    
    Ok(wav_data)
}
