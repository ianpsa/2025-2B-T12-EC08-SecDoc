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
        // Find the actual script path
        let actual_script = find_script(&script_path);
        
        println!("🔍 Looking for script: {:?}", actual_script);
        
        if !actual_script.exists() {
            eprintln!("❌ Script not found: {:?}", actual_script);
            eprintln!("   Current dir: {:?}", std::env::current_dir().ok());
            eprintln!("   Tried paths:");
            for p in get_script_candidates(&script_path) {
                eprintln!("     - {:?} (exists: {})", p, p.exists());
            }
            return None;
        }

        println!("✅ Found script: {:?}", actual_script);
        println!("🔌 Connecting to robot: {}", robot_ip);

        let mut child = match Command::new("python3")
            .arg(&actual_script)
            .arg(&robot_ip)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Failed to spawn Python: {}", e);
                return None;
            }
        };

        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;
        let stderr = child.stderr.take()?;

        let (done_tx, done_rx) = mpsc::channel::<()>(32);
        let (ready_tx, mut ready_rx) = mpsc::channel::<()>(1);

        // Spawn stderr reader for debugging
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                eprintln!("🐍 Python: {}", line.trim());
            }
        });

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
                println!("📤 Python stdout: {}", trimmed);
                
                if trimmed == "READY" && !ready_sent {
                    let _ = ready_tx.send(()).await;
                    ready_sent = true;
                } else if trimmed == "DONE" {
                    let _ = done_tx.send(()).await;
                }
            }
        });

        // Wait for READY signal
        println!("⏳ Waiting for Python READY signal...");
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(15),
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
                eprintln!("❌ Player timeout waiting for READY (15s)");
                eprintln!("   Check if robot is reachable at {}", robot_ip);
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

fn get_script_candidates(script_name: &PathBuf) -> Vec<PathBuf> {
    let name = script_name.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("play_audio.py");
    
    vec![
        PathBuf::from(name),
        PathBuf::from(format!("./{}", name)),
        PathBuf::from(format!("../{}", name)),
        PathBuf::from(format!("rust_audio_player/{}", name)),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join(name)))
            .unwrap_or_default(),
    ]
}

fn find_script(script_name: &PathBuf) -> PathBuf {
    for candidate in get_script_candidates(script_name) {
        if candidate.exists() {
            return candidate;
        }
    }
    script_name.clone()
}
