use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub serial_port: String,
    pub baud_rate: u32,
    pub ros_topic_name: String, // Renamed from service_name
    pub ros_namespace: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            serial_port: "/dev/ttyACM0".to_string(),
            baud_rate: 115200,
            ros_topic_name: "/api/sport/request".to_string(),
            ros_namespace: "emergency_stop".to_string(),
        }
    }
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        eprintln!("Arquivo de configuração não encontrado: {}. Usando valores padrão.", path);
        return Ok(Config::default());
    }
    
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}