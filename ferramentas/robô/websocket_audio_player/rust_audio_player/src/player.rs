use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

pub struct RobotPlayer {
    _process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    ready: Arc<Mutex<bool>>,
    done_rx: Arc<Mutex<mpsc::Receiver<()>>>,
}

impl RobotPlayer {
    pub async fn new(robot_ip: String, script_path: PathBuf) -> Option<Self> {
        info!("Starting Python streaming player...");

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

        let (done_tx, done_rx) = mpsc::channel::<()>(32);

        let player = Self {
            _process: Arc::new(Mutex::new(Some(child))),
            stdin: Arc::new(Mutex::new(Some(stdin))),
            ready: Arc::new(Mutex::new(false)),
            done_rx: Arc::new(Mutex::new(done_rx)),
        };

        let ready_flag = Arc::clone(&player.ready);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let msg = line.trim();
                        if msg == "READY" {
                            info!("Python streaming player ready");
                            *ready_flag.lock().await = true;
                        } else if msg == "DONE" {
                            let _ = done_tx.send(()).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        for _ in 0..100 {
            if *player.ready.lock().await {
                return Some(player);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        error!("Python player did not become ready");
        None
    }

    async fn send_command(&self, cmd: &str) {
        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            let _ = stdin.write_all(format!("{}\n", cmd).as_bytes()).await;
            let _ = stdin.flush().await;
        }
    }

    /// Start megaphone streaming mode
    pub async fn start_streaming(&self) {
        info!("Starting stream mode");
        self.send_command("START").await;
        // Small delay to let Python enter megaphone mode
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    /// Stream a single chunk (plays immediately in megaphone mode)
    pub async fn stream_chunk(&self, wav_path: &PathBuf) {
        if let Some(path_str) = wav_path.to_str() {
            self.send_command(&format!("CHUNK:{}", path_str)).await;
        }
    }

    /// Stop megaphone streaming mode
    pub async fn stop_streaming(&self) {
        self.send_command("STOP").await;
        // Wait for Python to confirm
        let mut rx = self.done_rx.lock().await;
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            rx.recv()
        ).await;
        info!("Stream complete");
    }
}
