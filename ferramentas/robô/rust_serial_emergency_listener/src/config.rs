use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub serial_port: String,
    pub baud_rate: u32,
    pub ros_service_name: String,
    pub ros_namespace: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            serial_port: "/dev/ttyACM0".to_string(),
            baud_rate: 115200,
            ros_service_name: "/go2/modes".to_string(),
            ros_namespace: "emergency_stop".to_string(),
        }
    }
}

/// Carrega configuração do arquivo YAML
pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        eprintln!("Arquivo de configuração não encontrado: {}. Usando valores padrão.", path);
        return Ok(Config::default());
    }
    
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}

/// Recarrega configuração (útil para atualizar sem reiniciar o serviço)
pub fn reload_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    load_config(path)
}