use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

use crate::audio::AudioProcessor;

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
                // Ignore DONE - we don't wait for it
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

/// Continuous streaming - sends all chunks without waiting
pub async fn stream_continuous(
    player: Arc<RobotPlayer>,
    mut wav_rx: mpsc::Receiver<PathBuf>,
    processor: Arc<AudioProcessor>,
) {
    let mut files_to_cleanup: Vec<PathBuf> = Vec::new();
    
    while let Some(wav_path) = wav_rx.recv().await {
        // Send immediately - don't wait
        player.send_audio(&wav_path).await;
        files_to_cleanup.push(wav_path);
        
        // Cleanup old files (keep last 5)
        while files_to_cleanup.len() > 5 {
            if let Some(old) = files_to_cleanup.first() {
                processor.cleanup(old).await;
            }
            files_to_cleanup.remove(0);
        }
    }
    
    // Final cleanup
    for f in files_to_cleanup {
        processor.cleanup(&f).await;
    }
}
