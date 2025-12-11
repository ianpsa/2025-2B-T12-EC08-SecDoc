use tokio::sync::mpsc;
use r2r::Publisher;
use crate::utils::{audio_decoder, ros_interface};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    publisher: Publisher<r2r::unitree_go::msg::AudioData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Audio processing pipeline started");
    
    while let Some(audio_data) = audio_receiver.recv().await {
        // Process audio (handles both encoded and raw PCM)
        match audio_decoder::process_audio(audio_data) {
            Ok(pcm_data) => {
                // Publish to ROS
                if let Err(e) = ros_interface::publish_audio(&publisher, pcm_data) {
                    eprintln!("Failed to publish audio: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to process audio: {}", e);
            }
        }
    }
    
    Ok(())
}
