// src/ros_audio_msg.rs
// Purpose: Define the Unitree AudioData message structure
// This mirrors: unitree_go/msg/AudioData.msg
//
// Teaching Notes:
// - This struct represents the ROS2 message format
// - Based on Unitree conventions, the topic "audiodata" uses lowercase
// - The message contains raw PCM audio bytes and a timestamp

use serde::{Deserialize, Serialize};

/// Unitree GO2 AudioData ROS2 Message
/// 
/// This represents the audio data message that the Unitree GO2 expects.
/// Based on analysis of unitree_ros2 patterns, this follows their message structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioData {
    /// Raw PCM audio bytes (S16LE format: signed 16-bit little-endian)
    /// This is the actual audio data the robot will play
    pub data: Vec<u8>,
    
    /// Timestamp frame (nanoseconds since epoch)
    /// Used for synchronization and ordering of audio packets
    pub time_frame: u64,
}

impl AudioData {
    /// Create a new AudioData message with current timestamp
    /// 
    /// # Arguments
    /// * `pcm_data` - Raw PCM audio bytes (interleaved S16LE format)
    /// 
    /// # Returns
    /// A new AudioData message ready to publish
    pub fn new(pcm_data: Vec<u8>) -> Self {
        // TODO: Get current system time as nanoseconds
        // HINT: You can use std::time::SystemTime for this
        // CODE_TEMPLATE:
        // use std::time::{SystemTime, UNIX_EPOCH};
        // let time_frame = SystemTime::now()
        //     .duration_since(UNIX_EPOCH)
        //     .unwrap()
        //     .as_nanos() as u64;
        
        // For now, using a simple placeholder
        // TODO: Uncomment the above code and replace this line
        let time_frame = 0;
        
        Self {
            data: pcm_data,
            time_frame,
        }
    }
    
    /// Create an AudioData message with a specific timestamp
    /// 
    /// # Arguments
    /// * `pcm_data` - Raw PCM audio bytes
    /// * `timestamp_nanos` - Timestamp in nanoseconds
    pub fn with_timestamp(pcm_data: Vec<u8>, timestamp_nanos: u64) -> Self {
        Self {
            data: pcm_data,
            time_frame: timestamp_nanos,
        }
    }
    
    /// Get the size of the audio data in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
    
    /// Check if the message contains audio data
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_data_creation() {
        let test_data = vec![0u8; 1024];
        let msg = AudioData::new(test_data.clone());
        
        assert_eq!(msg.data.len(), 1024);
        assert_eq!(msg.size(), 1024);
        assert!(!msg.is_empty());
    }
    
    #[test]
    fn test_audio_data_with_timestamp() {
        let test_data = vec![1u8, 2u8, 3u8];
        let timestamp = 12345678900u64;
        let msg = AudioData::with_timestamp(test_data, timestamp);
        
        assert_eq!(msg.time_frame, timestamp);
        assert_eq!(msg.data.len(), 3);
    }
}
