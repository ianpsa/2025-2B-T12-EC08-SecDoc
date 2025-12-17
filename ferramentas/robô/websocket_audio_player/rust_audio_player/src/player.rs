use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct RobotPlayer {
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    ready: Arc<Mutex<bool>>,
}

impl RobotPlayer {
    pub async fn new(robot_ip: String, script_path: PathBuf) -> Option<Self> {
        info!("Starting Python player process...");

        let mut child = match Command::new("python3")
            .arg(&script_path)
            .arg(&robot_ip)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to start Python: {}", e);
                return None;
            }
        };

        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        let player = Self {
            process: Arc::new(Mutex::new(Some(child))),
            stdin: Arc::new(Mutex::new(Some(stdin))),
            ready: Arc::new(Mutex::new(false)),
        };

        // Wait for "READY" signal
        let ready_flag = Arc::clone(&player.ready);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            while reader.read_line(&mut line).await.is_ok() {
                let msg = line.trim();
                if msg == "READY" {
                    info!("Python player ready");
                    *ready_flag.lock().await = true;
                } else if msg == "DONE" {
                    // Playback completed
                }
                line.clear();
            }
        });

        // Wait for ready
        for _ in 0..60 {
            if *player.ready.lock().await {
                return Some(player);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        error!("Python player did not become ready");
        None
    }

    pub async fn play_audio(&self, wav_path: &PathBuf) -> bool {
        let path_str = match wav_path.to_str() {
            Some(s) => s,
            None => return false,
        };

        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            let cmd = format!("{}\n", path_str);
            if stdin.write_all(cmd.as_bytes()).await.is_ok() {
                let _ = stdin.flush().await;
                info!("Sent to player: {}", wav_path.file_name().unwrap_or_default().to_string_lossy());
                return true;
            }
        }
        false
    }
}
