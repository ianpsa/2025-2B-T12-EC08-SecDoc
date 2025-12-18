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

    pub async fn decode(&self, audio_b64: &str, format: &str) -> Option<PathBuf> {
        let id = self.next_id();
        let input_path = self.temp_dir.join(format!("i{}.{}", id, format));
        let output_path = self.temp_dir.join(format!("o{}.wav", id));

        // Decode base64
        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .ok()?;
        tokio::fs::write(&input_path, &audio_bytes).await.ok()?;

        // FFmpeg conversion to WAV (16kHz mono for robot)
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-hide_banner",
                "-loglevel", "error",
                "-i", input_path.to_str()?,
                "-ar", "16000",
                "-ac", "1",
                "-acodec", "pcm_s16le",
                "-f", "wav",
                output_path.to_str()?,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        // Cleanup input
        let _ = tokio::fs::remove_file(&input_path).await;

        match status {
            Ok(s) if s.success() => Some(output_path),
            _ => None,
        }
    }

    pub fn cleanup(&self, path: &PathBuf) {
        let _ = std::fs::remove_file(path);
    }
}

impl Drop for AudioProcessor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}
