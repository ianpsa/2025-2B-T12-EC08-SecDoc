use r2r::unitree_api::msg::Request;
use r2r::{Node, Publisher, QosProfile};
use tracing::info;

pub struct RobotClient {
    pub publisher: Publisher<Request>,
    pub node: Node,
}

impl RobotClient {
    pub fn new(node_name: &str, topic_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!(
            "Initializing ROS2 client with r2r: node={}, topic={}",
            node_name, topic_name
        );

        let ctx = r2r::Context::create()?;
        let mut node = r2r::Node::create(ctx, node_name, "")?;
        let publisher = node.create_publisher::<Request>(topic_name, QosProfile::default())?;

        Ok(RobotClient { publisher, node })
    }

    pub async fn trigger_emote(&self, api_id: u32, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Emote command received! Triggering {}...", name);

        let msg = Request {
            header: r2r::unitree_api::msg::RequestHeader {
                identity: r2r::unitree_api::msg::RequestIdentity {
                    id: 123,
                    api_id: api_id.into(),
                },
                policy: r2r::unitree_api::msg::RequestPolicy {
                    priority: 1, // Lowest priority
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        self.publisher.publish(&msg)?;

        info!("{} emote sent successfully!", name);
        Ok(())
    }

}
