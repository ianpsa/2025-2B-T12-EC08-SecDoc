use r2r::unitree_api::msg::Request;
use r2r::{Node, Publisher, QosProfile};
use std::sync::{Arc, Mutex};
use tracing::{error, info};

pub struct EmergencyStopClient {
    pub publisher: Publisher<Request>,
    pub node: Node,
}

pub struct RecoverFromFall {
    pub publisher: Publisher<Request>,
    pub node: Node,
}

impl EmergencyStopClient {
    pub fn new(node_name: &str, topic_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!(
            "Initializing ROS2 client with r2r: node={}, topic={}",
            node_name, topic_name
        );

        let ctx = r2r::Context::create()?;
        let mut node = r2r::Node::create(ctx, node_name, "")?;
        let publisher = node.create_publisher::<Request>(topic_name, QosProfile::default())?;

        Ok(EmergencyStopClient { publisher, node })
    }

    pub async fn trigger_emergency_stop(&self, _state: bool) -> Result<(), Box<dyn std::error::Error>> {
        info!("Emergency button pressed! Triggering stop...");

        let msg = Request {
            header: r2r::unitree_api::msg::RequestHeader {
                identity: r2r::unitree_api::msg::RequestIdentity {
                    id: 123,
                    api_id: 1001, // DAMP
                },
                policy: r2r::unitree_api::msg::RequestPolicy {
                    priority: 255, // Maximum priority (u8)
                    noreply: false,
                },
                ..Default::default()
            },
            ..Default::default()
        };

        self.publisher.publish(&msg)?;

        error!("Emergency stop sent sucessfully!");
        Ok(())
    }

    pub async fn trigger_recovery(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Recovery command received! Triggering robot recovery...");

        let msg = Request {
            header: r2r::unitree_api::msg::RequestHeader {
                identity: r2r::unitree_api::msg::RequestIdentity {
                    id: 123,
                    api_id: 1006, // RECOVER
                },
                policy: r2r::unitree_api::msg::RequestPolicy {
                    priority: 255, 
                    noreply: false,
                },
                ..Default::default()
            },
            ..Default::default()
        };

        self.publisher.publish(&msg)?;

        info!("Recovery command sent successfully!");
        Ok(())
    }

    pub async fn start_spin_thread(
        node: Arc<Mutex<Node>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::thread::spawn(move || {
            loop {
                {
                    // lock only long enough to call spin_once
                    let mut node_locked = node.lock().unwrap();
                    node_locked.spin_once(std::time::Duration::from_millis(50));
                }
                // give other threads some room
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });

        Ok(())
    }
}

impl RecoverFromFall {
    pub fn new() {}
}
