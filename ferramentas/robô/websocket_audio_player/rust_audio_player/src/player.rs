use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

pub struct RobotPlayer {
    _process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    done_rx: Arc<Mutex<mpsc::Receiver<()>>>,
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

        let (done_tx, done_rx) = mpsc::channel::<()>(32);
        let (ready_tx, mut ready_rx) = mpsc::channel::<()>(1);

        // Spawn stdout reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut ready_sent = false;
            
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                
                let trimmed = line.trim();
                if trimmed == "READY" && !ready_sent {
                    let _ = ready_tx.send(()).await;
                    ready_sent = true;
                } else if trimmed == "DONE" {
                    let _ = done_tx.send(()).await;
                }
            }
        });

        // Wait for READY signal
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            ready_rx.recv()
        ).await;

        match timeout {
            Ok(Some(())) => {
                println!("✅ Player ready");
                Some(Self {
                    _process: Arc::new(Mutex::new(Some(child))),
                    stdin: Arc::new(Mutex::new(Some(stdin))),
                    done_rx: Arc::new(Mutex::new(done_rx)),
                })
            }
            _ => {
                eprintln!("❌ Player timeout waiting for READY");
                None
            }
        }
    }

    /// Send audio to robot and wait for completion
    pub async fn send_audio(&self, wav_path: &PathBuf) -> bool {
        if let Some(s) = wav_path.to_str() {
            // Send file path to Python script
            {
                let mut stdin = self.stdin.lock().await;
                if let Some(ref mut w) = *stdin {
                    if w.write_all(format!("{}\n", s).as_bytes()).await.is_err() {
                        return false;
                    }
                    let _ = w.flush().await;
                }
            }

            // Wait for DONE signal (with timeout)
            let mut rx = self.done_rx.lock().await;
            let timeout = tokio::time::timeout(
                std::time::Duration::from_secs(120), // 2 min max per audio
                rx.recv()
            ).await;

            return timeout.is_ok() && timeout.unwrap().is_some();
        }
        false
    }
}
