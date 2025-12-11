use tokio::sync::mpsc;
use r2r::Publisher;
use crate::utils::{audio_decoder, ros_interface};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    publisher: Publisher<r2r::unitree_go::msg::AudioData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Audio processing pipeline started");
    
    while let Some(audio_data) = audio_receiver.recv().await {
        println!("Received {} bytes of audio data", audio_data.len());
        
        // Process audio (handles both encoded and raw PCM)
        match audio_decoder::process_audio(audio_data) {
            Ok(pcm_data) => {
                println!("Processed {} bytes of PCM data", pcm_data.len());
                
                // Publish to ROS
                match ros_interface::publish_audio(&publisher, pcm_data) {
                    Ok(_) => {
                        println!("Successfully published audio to ROS");
                    }
                    Err(e) => {
                        eprintln!("Failed to publish audio: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to process audio: {}", e);
            }
        }
    }
    
    Ok(())
}
