use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct RobotPlayer {
    robot_ip: String,
    script_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl RobotPlayer {
    pub fn new(robot_ip: String, script_path: PathBuf) -> Self {
        Self {
            robot_ip,
            script_path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn play_audio(&self, wav_path: &PathBuf) -> bool {
        let _guard = self.lock.lock().await;

        info!("▶️  Playing: {:?}", wav_path.file_name().unwrap_or_default());

        let status = Command::new("python3")
            .arg(&self.script_path)
            .arg(&self.robot_ip)
            .arg(wav_path)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => {
                info!("✅ Playback complete");
                true
            }
            Ok(s) => {
                error!("Python script exited with: {}", s);
                false
            }
            Err(e) => {
                error!("Failed to run Python script: {}", e);
                false
            }
        }
    }
}

