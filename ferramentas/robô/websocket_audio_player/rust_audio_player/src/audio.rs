use base64::Engine;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::process::Command;
use tracing::{error, info};

pub struct AudioProcessor {
    temp_dir: PathBuf,
    counter: AtomicU64,
}

impl AudioProcessor {
    pub fn new() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let temp_dir = temp.path().to_path_buf();
        std::mem::forget(temp);
        info!("Temp: {:?}", temp_dir);
        Ok(Self { 
            temp_dir,
            counter: AtomicU64::new(0),
        })
    }

    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Decode base64 and convert to WAV
    pub async fn decode_and_convert(&self, audio_b64: &str, format: &str) -> Option<PathBuf> {
        let id = self.next_id();
        let input_path = self.temp_dir.join(format!("in_{}.{}", id, format));
        let output_path = self.temp_dir.join(format!("out_{}.wav", id));

        // Decode base64
        let audio_bytes = match base64::engine::general_purpose::STANDARD.decode(audio_b64) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Base64 error: {}", e);
                return None;
            }
        };

        // Write input file
        if let Err(e) = tokio::fs::write(&input_path, &audio_bytes).await {
            error!("Write error: {}", e);
            return None;
        }

        // Convert to WAV using ffmpeg
        let status = Command::new("ffmpeg")
            .args([
                "-y", "-i", input_path.to_str().unwrap(),
                "-ar", "44100", "-ac", "2", "-sample_fmt", "s16",
                "-f", "wav", output_path.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        // Cleanup input
        let _ = tokio::fs::remove_file(&input_path).await;

        match status {
            Ok(s) if s.success() && output_path.exists() => {
                info!("Converted: {}", output_path.file_name().unwrap().to_string_lossy());
                Some(output_path)
            }
            _ => {
                error!("FFmpeg failed");
                None
            }
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


