mod utils;

use tokio::sync::mpsc;
use utils::{ros_interface, websocket_server, streaming_pipeline};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting audio WebSocket to ROS service...");
    
    // Create ROS node and publisher
    let (mut node, publisher) = ros_interface::create_node_and_publisher()
        .map_err(|e| -> Box<dyn std::error::Error> { format!("Failed to create ROS node: {}", e).into() })?;
    
    // Create channel for communication between WebSocket and processing pipeline
    let (audio_sender, audio_receiver) = mpsc::channel::<Vec<u8>>(100);
    
    // Spawn ROS spinning task
    let node_handle = tokio::task::spawn_blocking(move || {
        loop {
            node.spin_once(std::time::Duration::from_millis(100));
        }
    });
    
    // Spawn audio processing pipeline
    let publisher_clone = publisher.clone();
    let pipeline_handle = tokio::spawn(async move {
        streaming_pipeline::start_pipeline(audio_receiver, publisher_clone).await
    });
    
    // Start WebSocket server (listens on all interfaces)
    let websocket_handle = tokio::spawn(async move {
        websocket_server::start_websocket_server("0.0.0.0:8080", audio_sender).await
    });
    
    // Wait for all tasks
    tokio::try_join!(node_handle, pipeline_handle, websocket_handle)?;
    
    Ok(())
}
