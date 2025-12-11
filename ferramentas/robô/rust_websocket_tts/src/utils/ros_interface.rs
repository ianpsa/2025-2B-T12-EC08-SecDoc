use r2r::{Node, Publisher, QosProfile};

pub fn create_node_and_publisher() -> Result<(Node, Publisher<r2r::unitree_go::msg::AudioData>), Box<dyn std::error::Error + Send + Sync>> {
    let ctx = r2r::Context::create()?;
    let mut node = Node::create(ctx, "audio_service_node", "")?;
    let publisher = node.create_publisher::<r2r::unitree_go::msg::AudioData>(
        "/audio_data",
        QosProfile::default()
    )?;
    Ok((node, publisher))
}

pub fn publish_audio(
    publisher: &Publisher<r2r::unitree_go::msg::AudioData>,
    audio_data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let msg = r2r::unitree_go::msg::AudioData {
        time_frame: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64,
        data: audio_data,
    };
    publisher.publish(&msg)?;
    Ok(())
}
