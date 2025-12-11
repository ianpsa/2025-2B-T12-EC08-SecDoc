use tokio::sync::mpsc;
use r2r::Publisher;
use crate::utils::{audio_decoder, ros_interface};

pub async fn start_pipeline(
    mut audio_receiver: mpsc::Receiver<Vec<u8>>,
    publisher: Publisher<unitree_go::msg::AudioData>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Audio processing pipeline started");
    
    while let Some(mp3_data) = audio_receiver.recv().await {
        // Decode MP3 to PCM
        match audio_decoder::decode_mp3_to_pcm(mp3_data) {
            Ok(pcm_data) => {
                // Publish to ROS
                if let Err(e) = ros_interface::publish_audio(&publisher, pcm_data) {
                    eprintln!("Failed to publish audio: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to decode audio: {}", e);
            }
        }
    }
    
    Ok(())
}
