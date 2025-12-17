use base64::Engine;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{error, info};

pub struct AudioProcessor {
    temp_dir: PathBuf,
}

impl AudioProcessor {
    pub fn new() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let temp_dir = temp.path().to_path_buf();
        std::mem::forget(temp); // Prevent automatic cleanup
        info!("Temp directory: {:?}", temp_dir);
        Ok(Self { temp_dir })
    }

    pub async fn decode_and_convert(&self, audio_b64: &str, format: &str) -> Option<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let input_path = self.temp_dir.join(format!("A{}.{}", timestamp, format));
        let output_path = self.temp_dir.join(format!("A{}.wav", timestamp));

        let audio_bytes = match base64::engine::general_purpose::STANDARD.decode(audio_b64) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Base64 decode failed: {}", e);
                return None;
            }
        };

        if let Err(e) = tokio::fs::write(&input_path, &audio_bytes).await {
            error!("Failed to write input file: {}", e);
            return None;
        }

        // Convert to WAV using ffmpeg (44.1kHz, stereo, 16-bit - compatible with Unitree GO2)
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", input_path.to_str().unwrap(),
                "-ar", "44100",
                "-ac", "2",
                "-sample_fmt", "s16",
                "-f", "wav",
                output_path.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        let _ = tokio::fs::remove_file(&input_path).await;

        match status {
            Ok(s) if s.success() => {
                if output_path.exists() {
                    info!("Converted: {:?}", output_path);
                    Some(output_path)
                } else {
                    error!("FFmpeg succeeded but output missing");
                    None
                }
            }
            Ok(s) => {
                error!("FFmpeg exited with: {}", s);
                None
            }
            Err(e) => {
                error!("FFmpeg failed: {}", e);
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


