pub struct Config {
    pub robot_ip: String,
    pub websocket_url: String,
    pub python_script: String,
}

impl Config {
    pub fn from_args() -> Result<Self, String> {
        let args: Vec<String> = std::env::args().collect();
        
        if args.len() < 3 {
            return Err(format!(
                "Usage: {} <robot_ip> <websocket_url>\nExample: {} 192.168.123.161 ws://10.8.250.17:8765",
                args[0], args[0]
            ));
        }

        Ok(Config {
            robot_ip: args[1].clone(),
            websocket_url: args[2].clone(),
            python_script: "play_audio.py".to_string(),
        })
    }
}

