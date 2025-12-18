use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub struct RobotPlayer {
    _process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    ready: Arc<Mutex<bool>>,
}

impl RobotPlayer {
    pub async fn new(robot_ip: String, script_path: PathBuf) -> Option<Self> {
        let mut child = Command::new("python3")
            .arg(&script_path)
            .arg(&robot_ip)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        let player = Self {
            _process: Arc::new(Mutex::new(Some(child))),
            stdin: Arc::new(Mutex::new(Some(stdin))),
            ready: Arc::new(Mutex::new(false)),
        };

        let ready_flag = Arc::clone(&player.ready);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                if line.trim() == "READY" {
                    *ready_flag.lock().await = true;
                }
            }
        });

        for _ in 0..50 {
            if *player.ready.lock().await {
                println!("Player ready");
                return Some(player);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        None
    }

    pub async fn send_audio(&self, wav_path: &PathBuf) -> bool {
        if let Some(s) = wav_path.to_str() {
            let mut stdin = self.stdin.lock().await;
            if let Some(ref mut w) = *stdin {
                if w.write_all(format!("{}\n", s).as_bytes()).await.is_ok() {
                    let _ = w.flush().await;
                    return true;
                }
            }
        }
        false
    }
}
