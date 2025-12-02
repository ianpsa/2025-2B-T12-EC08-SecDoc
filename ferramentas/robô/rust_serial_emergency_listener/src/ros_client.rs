use tracing::{info, error, warn};

pub struct EmergencyStopClient {
    service_name: String,
}

impl EmergencyStopClient {
    pub fn new(_node_name: &str, service_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing ROS2 client (command-line mode): service={}", service_name);
        
        Ok(EmergencyStopClient {
            service_name: service_name.to_string(),
        })
    }
    
    pub async fn trigger_emergency_stop(&self, state: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !state {
            return Ok(());
        }
        
        info!("Emergency button pressed! Triggering stop...");
        
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
    
    /// Mantém o nó ROS2 ativo (dummy - não necessário no modo command-line)
    pub async fn spin(&self) {
        // No modo command-line, não precisamos de spin
        // Apenas mantém a task viva
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
    
    /// Clone para uso em múltiplas tasks
    pub fn clone(&self) -> Self {
        EmergencyStopClient {
            service_name: self.service_name.clone(),
        }
    }
}