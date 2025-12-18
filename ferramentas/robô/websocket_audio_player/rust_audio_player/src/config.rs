use std::path::PathBuf;

/// Configuration for the audio player
pub struct Config {
    pub robot_ip: String,
    pub websocket_url: String,
    pub python_script: String,
    pub http_port: u16,
    pub audio_dir: PathBuf,
}

const DEFAULT_ROBOT_IP: &str = "192.168.123.161";
const DEFAULT_WEBSOCKET_URL: &str = "ws://0.0.0.0:8765";
const DEFAULT_HTTP_PORT: u16 = 8080;

impl Config {
    pub fn from_args() -> Result<Self, String> {
        let args: Vec<String> = std::env::args().collect();
        
        let robot_ip = args.get(1)
            .cloned()
            .unwrap_or_else(|| DEFAULT_ROBOT_IP.to_string());
        
        let websocket_url = args.get(2)
            .cloned()
            .unwrap_or_else(|| DEFAULT_WEBSOCKET_URL.to_string());

        let http_port = args.get(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_HTTP_PORT);

        // Audio directory - look for ../audio relative to executable or current dir
        let audio_dir = find_audio_dir();

        Ok(Config {
            robot_ip,
            websocket_url,
            python_script: "play_audio.py".to_string(),
            http_port,
            audio_dir,
        })
    }
}

fn find_audio_dir() -> PathBuf {
    // Try relative to current directory
    let candidates = vec![
        PathBuf::from("../audio"),
        PathBuf::from("audio"),
        PathBuf::from("../../audio"),
    ];

    for path in candidates {
        if path.exists() && path.is_dir() {
            return path;
        }
    }

    // Default to ../audio
    PathBuf::from("../audio")
}
