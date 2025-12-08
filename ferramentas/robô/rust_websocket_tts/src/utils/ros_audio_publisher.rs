// src/utils/ros_audio_publisher.rs
// Purpose: Publish PCM audio to Unitree GO2 via ROS2
//
// Teaching Notes:
// - ROS2 is like a message bus/postal service for robots
// - We create a "publisher" that sends messages to a "topic"
// - The Unitree GO2 "subscribes" to that topic and plays the audio
// - Based on unitree_ros2 patterns, the topic is likely "audiodata" (lowercase)
//
// Analogy: Think of ROS2 like a radio station:
//   - We're the DJ broadcasting (publishing)
//   - The robot is listening to our frequency (subscribed to topic)
//   - The audio data is the music we're broadcasting

use anyhow::{Result, Context, anyhow};
use tracing::{info, error, warn};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// R2R imports for ROS2 functionality
use r2r::QosProfile;

use crate::ros_audio_msg::AudioData;

/// ROS2 Audio Publisher for Unitree GO2
/// 
/// This struct manages the ROS2 connection and publishes audio data
/// to the robot's audio playback system.
pub struct RosAudioPublisher {
    /// ROS2 node (wrapped in Arc<Mutex<>> for thread safety)
    /// The node is like our "radio station" - it manages all ROS2 communication
    node: Arc<Mutex<r2r::Node>>,
    
    /// Audio data publisher
    /// This is the "transmitter" that sends audio messages
    publisher: Arc<Mutex<r2r::Publisher<r2r::std_msgs::msg::ByteMultiArray>>>,
    
    /// Topic name we're publishing to (e.g., "audiodata")
    topic_name: String,
}

impl RosAudioPublisher {
    /// Create a new ROS2 audio publisher
    /// 
    /// # Arguments
    /// * `node_name` - Name of this ROS2 node (e.g., "rust_audio_publisher")
    ///                 This appears in `ros2 node list`
    /// * `topic_name` - ROS2 topic to publish on (e.g., "audiodata")
    ///                  This is where the robot listens
    /// 
    /// # Returns
    /// * `Result<Self>` - Initialized ROS2 publisher or error
    pub fn new(node_name: &str, topic_name: &str) -> Result<Self> {
        info!("🤖 Initializing ROS2 audio publisher");
        info!("   Node name: {}", node_name);
        info!("   Topic: {}", topic_name);
        
        // ====================================================================
        // STEP 1: Create ROS2 context
        // ====================================================================
        // The context is like the "ROS2 environment" - it manages the connection
        // TODO: Uncomment and use:
        // let ctx = r2r::Context::create()
        //     .context("Failed to create ROS2 context")?;
        
        let ctx = r2r::Context::create()
            .context("Failed to create ROS2 context - is ROS2 installed?")?;
        
        info!("✓ ROS2 context created");
        
        // ====================================================================
        // STEP 2: Create ROS2 node
        // ====================================================================
        // The node is like our "radio station" - it's our presence in the ROS2 network
        // TODO: Uncomment:
        // let mut node = r2r::Node::create(ctx, node_name, "")
        //     .context("Failed to create ROS2 node")?;
        
        let mut node = r2r::Node::create(ctx, node_name, "")
            .context("Failed to create ROS2 node")?;
        
        info!("✓ ROS2 node '{}' created", node_name);
        
        // ====================================================================
        // STEP 3: Create publisher for audio data
        // ====================================================================
        // The publisher is our "transmitter" - it sends messages to the topic
        // 
        // IMPORTANT: We're using ByteMultiArray as a generic message type
        // This is flexible and works with most ROS2 systems
        // 
        // TODO: Uncomment and use:
        // let publisher = node.create_publisher::<r2r::std_msgs::msg::ByteMultiArray>(
        //     topic_name,
        //     QosProfile::default()
        // )?;
        
        let publisher = node.create_publisher::<r2r::std_msgs::msg::ByteMultiArray>(
            topic_name,
            QosProfile::default()
        )?;
        
        info!("✓ Publisher created on topic '{}'", topic_name);
        
        // ====================================================================
        // STEP 4: Wrap in Arc<Mutex<>> for thread safety
        // ====================================================================
        // We need this because tokio (async) might access from different threads
        // TODO: Uncomment:
        // let node = Arc::new(Mutex::new(node));
        // let publisher = Arc::new(Mutex::new(publisher));
        
        let node = Arc::new(Mutex::new(node));
        let publisher = Arc::new(Mutex::new(publisher));
        
        info!("🎉 ROS2 audio publisher ready!");
        
        Ok(Self {
            node,
            publisher,
            topic_name: topic_name.to_string(),
        })
    }
    
    /// Publish PCM audio data to ROS2
    /// 
    /// # Arguments
    /// * `pcm_bytes` - Raw PCM audio data (S16LE format)
    /// 
    /// # Returns
    /// * `Result<()>` - Success or error
    /// 
    /// # Example Flow:
    /// ```
    /// PCM bytes → ByteMultiArray message → ROS2 publish → Robot plays
    /// ```
    pub async fn publish_audio(&self, pcm_bytes: Vec<u8>) -> Result<()> {
        info!("📤 Publishing audio to topic '{}': {} bytes", 
              self.topic_name, pcm_bytes.len());
        
        // ====================================================================
        // STEP 1: Create a ByteMultiArray message
        // ====================================================================
        // This is a generic ROS2 message type that can hold any byte array
        // TODO: Uncomment and use:
        // let mut msg = r2r::std_msgs::msg::ByteMultiArray::default();
        // msg.data = pcm_bytes;
        
        let mut msg = r2r::std_msgs::msg::ByteMultiArray::default();
        msg.data = pcm_bytes;
        
        // ====================================================================
        // STEP 2: Publish the message
        // ====================================================================
        // This sends the message to the ROS2 topic
        // TODO: Uncomment:
        // let publisher = self.publisher.lock()
        //     .map_err(|e| anyhow!("Failed to lock publisher: {}", e))?;
        // 
        // publisher.publish(&msg)
        //     .context("Failed to publish audio message")?;
        
        let publisher = self.publisher.lock()
            .map_err(|e| anyhow!("Failed to lock publisher: {}", e))?;
        
        publisher.publish(&msg)
            .context("Failed to publish audio message")?;
        
        info!("✓ Audio published successfully");
        
        // ====================================================================
        // STEP 3: Spin node to process callbacks
        // ====================================================================
        // This gives ROS2 a chance to process the message
        // TODO: Uncomment:
        // self.spin_once()?;
        
        self.spin_once()?;
        
        Ok(())
    }
    
    /// Process ROS2 callbacks once
    /// 
    /// This should be called periodically to keep ROS2 communication alive.
    /// It's like "checking the mailbox" for incoming messages and
    /// "sending outgoing mail".
    pub fn spin_once(&self) -> Result<()> {
        // TODO: Lock the node and call spin_once
        // CODE_TEMPLATE:
        // let mut node = self.node.lock()
        //     .map_err(|e| anyhow!("Failed to lock node: {}", e))?;
        // 
        // node.spin_once(Duration::from_millis(10));
        
        let mut node = self.node.lock()
            .map_err(|e| anyhow!("Failed to lock node: {}", e))?;
        
        node.spin_once(Duration::from_millis(10));
        
        Ok(())
    }
    
    /// Get the topic name this publisher is using
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }
}

// Implement Clone if needed (Arc makes this easy)
impl Clone for RosAudioPublisher {
    fn clone(&self) -> Self {
        Self {
            node: Arc::clone(&self.node),
            publisher: Arc::clone(&self.publisher),
            topic_name: self.topic_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publisher_creation() {
        // This test will only work if ROS2 is installed
        // It's okay if it fails in CI/CD
        match RosAudioPublisher::new("test_node", "test_topic") {
            Ok(pub_) => {
                assert_eq!(pub_.topic_name(), "test_topic");
            }
            Err(e) => {
                // Expected if ROS2 is not available
                eprintln!("ROS2 not available (expected in non-ROS environment): {}", e);
            }
        }
    }
}
