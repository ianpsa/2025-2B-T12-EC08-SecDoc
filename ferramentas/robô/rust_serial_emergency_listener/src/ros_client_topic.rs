// Versão alternativa do ROS client usando TÓPICOS ao invés de SERVIÇOS
// Use este arquivo se o go2_ros2_toolbox usar tópicos para controle

use r2r::{Context, Node, QosProfile};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, warn};

pub struct EmergencyStopClient {
    node: Arc<Mutex<Node>>,
    topic_name: String,
}

impl EmergencyStopClient {
    pub fn new(node_name: &str, topic_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing ROS2 client (topic mode): node={}, topic={}", node_name, topic_name);
        
        let ctx = Context::create()?;
        let node = Node::create(ctx, node_name, "")?;
        
        info!("ROS2 Node Created");
        
        Ok(EmergencyStopClient {
            node: Arc::new(Mutex::new(node)),
            topic_name: topic_name.to_string(),
        })
    }
    
    pub async fn trigger_emergency_stop(&self, state: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !state {
            return Ok(());
        }
        
        info!("Emergency button pressed! Publishing to topic: {}", self.topic_name);
        
        // Opção 1: Usar comando ros2 topic pub
        let output = tokio::process::Command::new("ros2")
            .args(&[
                "topic",
                "pub",
                "--once",
                &self.topic_name,
                "std_msgs/msg/String",
                "{data: 'emergency_stop'}",
            ])
            .output()
            .await;
        
        match output {
            Ok(result) => {
                if result.status.success() {
                    info!("Emergency message published successfully");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    error!("Failed to publish message: {}", stderr);
                    Err(format!("Topic publish error: {}", stderr).into())
                }
            }
            Err(e) => {
                error!("Failed to execute ros2 command: {}", e);
                Err(e.into())
            }
        }
    }
    
    /// Mantém o nó ROS2 ativo (spinning)
    pub async fn spin(&self) {
        loop {
            {
                let mut node = self.node.lock().await;
                node.spin_once(std::time::Duration::from_millis(100));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
    
    /// Clone do Arc para uso em múltiplas tasks
    pub fn clone(&self) -> Self {
        EmergencyStopClient {
            node: Arc::clone(&self.node),
            topic_name: self.topic_name.clone(),
        }
    }
}

