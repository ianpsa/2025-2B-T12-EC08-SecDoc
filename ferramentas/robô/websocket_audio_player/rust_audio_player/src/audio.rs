use base64::Engine;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::process::Command;

pub struct AudioProcessor {
    temp_dir: PathBuf,
    counter: AtomicU64,
}

impl AudioProcessor {
    pub fn new() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let temp_dir = temp.path().to_path_buf();
        std::mem::forget(temp);
        Ok(Self { 
            temp_dir,
            counter: AtomicU64::new(0),
        })
    }

    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Decode base64 and convert to WAV optimized for robot
    pub async fn decode_and_convert(&self, audio_b64: &str, format: &str) -> Option<PathBuf> {
        let id = self.next_id();
        let input_path = self.temp_dir.join(format!("in_{}.{}", id, format));
        let output_path = self.temp_dir.join(format!("out_{}.wav", id));

        // Decode base64
        let audio_bytes = base64::engine::general_purpose::STANDARD.decode(audio_b64).ok()?;

        // Write input file
        tokio::fs::write(&input_path, &audio_bytes).await.ok()?;

        // Convert to WAV with robot-optimal settings (16kHz mono)
        // Using faster conversion settings
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", input_path.to_str().unwrap(),
                "-ar", "16000",      // 16kHz for robot
                "-ac", "1",          // Mono
                "-sample_fmt", "s16",
                "-f", "wav",
                output_path.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        // Cleanup input immediately
        let _ = tokio::fs::remove_file(&input_path).await;

        match status {
            Ok(s) if s.success() && output_path.exists() => Some(output_path),
            _ => None,
        }
    }

    pub async fn cleanup(&self, path: &PathBuf) {
        let _ = tokio::fs::remove_file(path).await;
    }
}

impl Drop for AudioProcessor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}
