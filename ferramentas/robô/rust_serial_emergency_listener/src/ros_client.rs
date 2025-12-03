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
        
        // Timeout de 1.5 segundos ao comando (reduzido para resposta rápida)
        let output = tokio::time::timeout(
            tokio::time::Duration::from_millis(200),
            tokio::process::Command::new("ros2")
                .args(&[
                    "service",
                    "call",
                    &self.service_name,
                    "go2_srvs/srv/Go2Modes",
                    "{request_data: damp}",
                ])
                .output()
        )
        .await;
        
        match output {
            Ok(Ok(result)) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    info!("Emergency service called successfully. Response: {}", stdout.trim());
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    error!("Failed to call service: {}", stderr);
                    Err(format!("Service returned error: {}", stderr).into())
                }
            }
            Ok(Err(e)) => {
                error!("Failed to execute ros2 command: {}", e);
                Err(e.into())
            }
            Err(_) => {
                error!("ROS2 service call timed out after 200 milliseconds");
                Err("Service call timeout".into())
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