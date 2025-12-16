use tokio::sync::mpsc;
use r2r::Publisher;
use crate::utils::{audio_decoder, ros_interface};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    publisher: Publisher<r2r::unitree_go::msg::AudioData>,
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
                
                // Publish to ROS
                println!("[PIPELINE] Sending to ROS publisher...");
                match ros_interface::publish_audio(&publisher, pcm_data) {
                    Ok(_) => {
                        println!("[PIPELINE] ✓ Message #{} fully processed and published", message_count);
                    }
                    Err(e) => {
                        eprintln!("[PIPELINE] ✗ Failed to publish audio for message #{}: {}", message_count, e);
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
