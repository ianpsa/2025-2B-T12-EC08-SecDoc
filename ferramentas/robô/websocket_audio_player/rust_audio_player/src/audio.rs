use base64::Engine;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Size of audio chunks for streaming (in bytes of decoded audio)
/// 2 seconds per chunk = good balance between latency and overhead
const CHUNK_DURATION_MS: u64 = 2000; // 2 second chunks
const SAMPLE_RATE: u64 = 44100;
const CHANNELS: u64 = 2;
const BYTES_PER_SAMPLE: u64 = 2;
const CHUNK_SIZE: u64 = (SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE * CHUNK_DURATION_MS) / 1000;

pub struct AudioProcessor {
    temp_dir: PathBuf,
    chunk_counter: std::sync::atomic::AtomicU64,
}

impl AudioProcessor {
    pub fn new() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let temp_dir = temp.path().to_path_buf();
        std::mem::forget(temp);
        info!("Temp directory: {:?}", temp_dir);
        Ok(Self { 
            temp_dir,
            chunk_counter: std::sync::atomic::AtomicU64::new(0),
        })
    }

    fn next_id(&self) -> u64 {
        self.chunk_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Decode base64 audio to raw file
    pub async fn decode_to_file(&self, audio_b64: &str, format: &str) -> Option<PathBuf> {
        let id = self.next_id();
        let input_path = self.temp_dir.join(format!("in_{}.{}", id, format));

        let audio_bytes = match base64::engine::general_purpose::STANDARD.decode(audio_b64) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Base64 decode failed: {}", e);
                return None;
            }
        };

        if let Err(e) = tokio::fs::write(&input_path, &audio_bytes).await {
            error!("Failed to write file: {}", e);
            return None;
        }

        Some(input_path)
    }

    /// Convert audio file to WAV and split into streaming chunks
    /// Returns a channel that yields WAV chunk paths as they're ready
    pub async fn convert_to_streaming_chunks(
        &self,
        input_path: &PathBuf,
    ) -> mpsc::Receiver<PathBuf> {
        let (tx, rx) = mpsc::channel::<PathBuf>(16);
        let temp_dir = self.temp_dir.clone();
        let input = input_path.clone();
        let base_id = self.next_id();

        tokio::spawn(async move {
            // First, convert entire file to raw PCM
            let raw_path = temp_dir.join(format!("raw_{}.pcm", base_id));
            
            let status = Command::new("ffmpeg")
                .args([
                    "-y", "-i", input.to_str().unwrap(),
                    "-ar", "44100", "-ac", "2", "-f", "s16le",
                    raw_path.to_str().unwrap(),
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;

            // Clean up input file
            let _ = tokio::fs::remove_file(&input).await;

            if status.is_err() || !status.unwrap().success() {
                error!("FFmpeg conversion failed");
                return;
            }

            // Read raw PCM data
            let pcm_data = match tokio::fs::read(&raw_path).await {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read PCM: {}", e);
                    return;
                }
            };
            let _ = tokio::fs::remove_file(&raw_path).await;

            let chunk_size = CHUNK_SIZE as usize;
            let total_chunks = (pcm_data.len() + chunk_size - 1) / chunk_size;
            
            info!("Splitting into {} chunks ({} bytes each)", total_chunks, chunk_size);

            // Process chunks and send them as they're ready
            for (i, chunk) in pcm_data.chunks(chunk_size).enumerate() {
                let chunk_path = temp_dir.join(format!("chunk_{}_{}.wav", base_id, i));
                
                // Write WAV header + PCM data
                if let Ok(()) = write_wav_file(&chunk_path, chunk, 44100, 2).await {
                    if tx.send(chunk_path).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            }
            
            info!("All {} chunks ready", total_chunks);
        });

        rx
    }

    /// Simple single-file conversion (fallback)
    pub async fn decode_and_convert(&self, audio_b64: &str, format: &str) -> Option<PathBuf> {
        let input_path = self.decode_to_file(audio_b64, format).await?;
        let id = self.next_id();
        let output_path = self.temp_dir.join(format!("out_{}.wav", id));

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

/// Write raw PCM data as a WAV file
async fn write_wav_file(path: &PathBuf, pcm_data: &[u8], sample_rate: u32, channels: u16) -> std::io::Result<()> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_size = pcm_data.len() as u32;
    let file_size = 36 + data_size;

    let mut wav_data = Vec::with_capacity(44 + pcm_data.len());
    
    // RIFF header
    wav_data.extend_from_slice(b"RIFF");
    wav_data.extend_from_slice(&file_size.to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");
    
    // fmt chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes());  // PCM format
    wav_data.extend_from_slice(&channels.to_le_bytes());
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());
    
    // data chunk
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&data_size.to_le_bytes());
    wav_data.extend_from_slice(pcm_data);

    tokio::fs::write(path, wav_data).await
}


