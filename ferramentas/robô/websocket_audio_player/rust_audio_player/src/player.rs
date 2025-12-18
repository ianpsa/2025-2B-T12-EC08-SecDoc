use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

pub struct RobotPlayer {
    _process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    ready: Arc<Mutex<bool>>,
    done_rx: Arc<Mutex<mpsc::Receiver<()>>>,
}

impl RobotPlayer {
    pub async fn new(robot_ip: String, script_path: PathBuf) -> Option<Self> {
        let mut child = Command::new("python3")
            .arg(&script_path)
            .arg(&robot_ip)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())  // Suppress stderr
            .spawn()
            .ok()?;

        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        let (done_tx, done_rx) = mpsc::channel::<()>(64);

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
                            *ready_flag.lock().await = true;
                        } else if msg == "DONE" {
                            let _ = done_tx.send(()).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Wait for Python to be ready
        for _ in 0..50 {
            if *player.ready.lock().await {
                println!("Player ready");
                return Some(player);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        eprintln!("❌ Player timeout");
        None
    }

    pub async fn play_audio(&self, wav_path: &PathBuf) -> bool {
        let path_str = match wav_path.to_str() {
            Some(s) => s,
            None => return false,
        };

        // Drain stale DONE messages
        {
            let mut rx = self.done_rx.lock().await;
            while rx.try_recv().is_ok() {}
        }

        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            if stdin.write_all(format!("{}\n", path_str).as_bytes()).await.is_ok() {
                let _ = stdin.flush().await;
                drop(stdin_guard);

                // Wait for completion with shorter timeout
                let mut rx = self.done_rx.lock().await;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    rx.recv()
                ).await {
                    Ok(Some(())) => return true,
                    _ => return false,
                }
            }
        }
        false
    }
}
