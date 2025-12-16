use r2r::{Node, Publisher, QosProfile};

pub fn create_node_and_publisher() -> Result<(Node, Publisher<r2r::unitree_go::msg::AudioData>), Box<dyn std::error::Error + Send + Sync>> {
    println!("[ROS] Creating ROS2 context...");
    let ctx = r2r::Context::create()?;
    println!("[ROS] Creating node 'audio_service_node'...");
    let mut node = Node::create(ctx, "audio_service_node", "")?;
    println!("[ROS] Creating publisher on topic '/audio_data'...");
    let publisher = node.create_publisher::<r2r::unitree_go::msg::AudioData>(
        "/audio_data",
        QosProfile::default()
    )?;
    println!("[ROS] Publisher created successfully on topic '/audio_data'");
    println!("[ROS] Node and publisher initialization complete");
    Ok((node, publisher))
}

pub fn publish_audio(
    publisher: &Publisher<r2r::unitree_go::msg::AudioData>,
    audio_data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_size = audio_data.len();
    println!("[ROS] Preparing to publish audio message...");
    println!("[ROS]   - Data size: {} bytes", data_size);
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    
    println!("[ROS]   - Timestamp: {}", timestamp);
    
    let msg = r2r::unitree_go::msg::AudioData {
        time_frame: timestamp,
        data: audio_data,
    };
    
    println!("[ROS] Publishing message to '/audio_data' topic...");
    match publisher.publish(&msg) {
        Ok(_) => {
            println!("[ROS] ✓ Message published successfully!");
            println!("[ROS]   - Topic: /audio_data");
            println!("[ROS]   - Payload size: {} bytes", data_size);
            println!("[ROS]   - Timestamp: {}", timestamp);
            Ok(())
        }
        Err(e) => {
            eprintln!("[ROS] ✗ Failed to publish message: {}", e);
            Err(e.into())
        }
    }
}
