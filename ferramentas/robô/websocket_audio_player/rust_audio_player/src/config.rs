/// Configuração do player de áudio
pub struct Config {
    pub robot_ip: String,
    pub websocket_url: String,
    pub python_script: String,
}

/// IP padrão do robô Unitree GO2
const DEFAULT_ROBOT_IP: &str = "192.168.123.161";
/// WebSocket padrão para receber áudio
const DEFAULT_WEBSOCKET_URL: &str = "ws://0.0.0.0:8765";

impl Config {
    pub fn from_args() -> Result<Self, String> {
        let args: Vec<String> = std::env::args().collect();
        
        let robot_ip = args.get(1)
            .cloned()
            .unwrap_or_else(|| DEFAULT_ROBOT_IP.to_string());
        
        let websocket_url = args.get(2)
            .cloned()
            .unwrap_or_else(|| DEFAULT_WEBSOCKET_URL.to_string());

        Ok(Config {
            robot_ip,
            websocket_url,
            python_script: "play_audio.py".to_string(),
        })
    }
}

