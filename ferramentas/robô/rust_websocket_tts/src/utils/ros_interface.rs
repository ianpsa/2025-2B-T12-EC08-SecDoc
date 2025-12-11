use r2r::{Node, Publisher, QosProfile};

pub fn create_node_and_publisher() -> Result<(Node, Publisher<unitree_go::msg::AudioData>), Box<dyn std::error::Error>> {
    let ctx = r2r::Context::create()?;
    let mut node = Node::create(ctx, "audio_service_node", "")?;
    let publisher = node.create_publisher::<unitree_go::msg::AudioData>(
        "/audio_data",
        QosProfile::default()
    )?;
    Ok((node, publisher))
}

pub fn publish_audio(
    publisher: &Publisher<unitree_go::msg::AudioData>,
    audio_data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = unitree_go::msg::AudioData {
        time_frame: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64,
        data: audio_data,
    };
    publisher.publish(&msg)?;
    Ok(())
}
