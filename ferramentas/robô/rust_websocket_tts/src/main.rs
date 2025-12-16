mod utils;

use tokio::sync::mpsc;
use utils::{ros_interface, websocket_server, streaming_pipeline};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==============================================");
    println!("  Audio WebSocket to ROS Service");
    println!("==============================================");
    println!();
    
    // Create ROS node and publisher
    println!("Step 1: Initializing ROS2 node and publisher...");
    let (mut node, publisher) = ros_interface::create_node_and_publisher()
        .map_err(|e| -> Box<dyn std::error::Error> { format!("Failed to create ROS node: {}", e).into() })?;
    println!();
    
    // Create channel for communication between WebSocket and processing pipeline
    println!("Step 2: Creating audio processing channel...");
    let (audio_sender, audio_receiver) = mpsc::channel::<Vec<u8>>(100);
    println!("  ✓ Channel created with buffer size: 100");
    println!();
    
    // Spawn ROS spinning task
    println!("Step 3: Starting ROS node spinning task...");
    let node_handle = tokio::task::spawn_blocking(move || {
        println!("[ROS SPIN] ROS node spinning started");
        let mut spin_count = 0;
        loop {
            node.spin_once(std::time::Duration::from_millis(100));
            spin_count += 1;
            if spin_count % 100 == 0 {
                println!("[ROS SPIN] Node still spinning (count: {})", spin_count);
            }
        }
    });
    println!("  ✓ ROS spinning task started");
    println!();
    
    // Spawn audio processing pipeline
    println!("Step 4: Starting audio processing pipeline...");
    let publisher_clone = publisher.clone();
    let pipeline_handle = tokio::spawn(async move {
        streaming_pipeline::start_pipeline(audio_receiver, publisher_clone).await
    });
    println!("  ✓ Pipeline task started");
    println!();
    
    // Start WebSocket server (listens on all interfaces)
    println!("Step 5: Starting WebSocket server...");
    let websocket_handle = tokio::spawn(async move {
        websocket_server::start_websocket_server("0.0.0.0:8080", audio_sender).await
    });
    println!("  ✓ WebSocket server task started on 0.0.0.0:8080");
    println!();
    
    println!("==============================================");
    println!("  Service is ready!");
    println!("==============================================");
    println!("  WebSocket: ws://0.0.0.0:8080");
    println!("  ROS Topic: /audio_data");
    println!();
    println!("Debug commands:");
    println!("  ros2 topic list");
    println!("  ros2 topic echo /audio_data");
    println!("  ros2 topic hz /audio_data");
    println!("  ros2 node list");
    println!("==============================================");
    println!();
    
    // Wait for all tasks
    tokio::try_join!(node_handle, pipeline_handle, websocket_handle)?;
    
    Ok(())
}
