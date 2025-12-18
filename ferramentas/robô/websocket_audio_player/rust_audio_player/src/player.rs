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

        let (done_tx, done_rx) = mpsc::channel::<()>(128);

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

        for _ in 0..50 {
            if *player.ready.lock().await {
                println!("Player ready");
                return Some(player);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        eprintln!("Player timeout");
        None
    }

    /// Send a WAV file to be played (non-blocking - doesn't wait for completion)
    pub async fn send_audio(&self, wav_path: &PathBuf) -> bool {
        let path_str = match wav_path.to_str() {
            Some(s) => s,
            None => return false,
        };

        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            if stdin.write_all(format!("{}\n", path_str).as_bytes()).await.is_ok() {
                let _ = stdin.flush().await;
                return true;
            }
        }
        false
    }

    /// Wait for one playback to complete
    pub async fn wait_done(&self) -> bool {
        let mut rx = self.done_rx.lock().await;
        // Drain any stale messages first
        while rx.try_recv().is_ok() {}
        
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            rx.recv()
        ).await {
            Ok(Some(())) => true,
            _ => false,
        }
    }
}

/// Continuous playback - sends chunks to Python without waiting between them
pub async fn play_continuous(
    player: Arc<RobotPlayer>,
    mut wav_rx: mpsc::Receiver<PathBuf>,
    processor: Arc<AudioProcessor>,
) {
    let mut pending_files: Vec<PathBuf> = Vec::new();
    
    while let Some(wav_path) = wav_rx.recv().await {
        // Send to player immediately
        player.send_audio(&wav_path).await;
        pending_files.push(wav_path);
        
        // Check if there are more chunks ready (non-blocking)
        while let Ok(next_path) = wav_rx.try_recv() {
            player.send_audio(&next_path).await;
            pending_files.push(next_path);
        }
        
        // Wait for one completion and cleanup
        if player.wait_done().await {
            if let Some(done_file) = pending_files.first() {
                processor.cleanup(done_file).await;
            }
            if !pending_files.is_empty() {
                pending_files.remove(0);
            }
        }
    }
    
    // Cleanup remaining files
    for file in pending_files {
        processor.cleanup(&file).await;
    }
}
