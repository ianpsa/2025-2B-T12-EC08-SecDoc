use r2r::{Context, Node};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, warn};

pub struct EmergencyStopClient {
    node: Arc<Mutex<Node>>,
    service_name: String,
}

impl EmergencyStopClient {
    pub fn new(node_name: &str, service_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing ROS2 client: node={}, service={}", node_name, service_name);
        
        let ctx = Context::create()?;
        let node = Node::create(ctx, node_name, "")?;
        
        info!("ROS2 Nodes Created");
        
        Ok(EmergencyStopClient {
            node: Arc::new(Mutex::new(node)),
            service_name: service_name.to_string(),
        })
    }
    
    pub async fn trigger_emergency_stop(&self, state: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !state {
            return Ok(());
        }
                
        let _node = self.node.lock().await;
        
        let output = tokio::process::Command::new("ros2")
            .args(&[
                "service",
                "call",
                &self.service_name,
                "go2_srvs/srv/Go2Modes",
                "{request_data: damp}",
            ])
            .output()
            .await;
        
        match output {
            Ok(result) => {
                if result.status.success() {
                    info!("Emergency service called, success");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    error!("Failed to call service: {}", stderr);
                    warn!("Emergency button is taking too much to send the messages");
                    Err(format!("Service returned error: {}", stderr).into())
                }
            }
            Err(e) => {
                error!("Failed to send the robot a command: {}", e);
                warn!("Emergency button is taking too much to send the messages");
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
            service_name: self.service_name.clone(),
        }
    }
}