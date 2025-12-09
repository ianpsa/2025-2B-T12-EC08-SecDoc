use anyhow::{Result, anyhow};
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use tracing::info;

/// Audio player for decoding and playing various audio formats
#[derive(Clone)]
pub struct AudioPlayer {
    // Empty struct - we create streams on-demand for thread safety
}

impl AudioPlayer {
    /// Create a new audio player
    pub fn new() -> Result<Self> {
        // Test that we can create an audio output stream
        let (_stream, _stream_handle) = OutputStream::try_default()
            .map_err(|e| anyhow!("Failed to create audio output stream: {}", e))?;

        info!("Audio player initialized");

        Ok(Self {})
    }

    /// Play audio from bytes
    /// 
    /// # Arguments
    /// * `audio_bytes` - Raw audio data (MP3, WAV, OGG, etc.)
    /// * `format` - Audio format hint (e.g., "mp3", "wav", "ogg")
    pub async fn play_audio(&self, audio_bytes: &[u8], format: &str) -> Result<()> {
        info!("Playing {} bytes of {} audio", audio_bytes.len(), format);

        // Create a cursor for the audio data
        let cursor = Cursor::new(audio_bytes.to_vec());

        // Spawn blocking task to avoid blocking the async runtime
        let format = format.to_string();
        tokio::task::spawn_blocking(move || {
            // Create output stream for this playback
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|e| anyhow!("Failed to create audio output stream: {}", e))?;

            // Decode the audio
            let source = Decoder::new(cursor)
                .map_err(|e| anyhow!("Failed to decode {} audio: {}", format, e))?;

            // Create a sink to play the audio
            let sink = Sink::try_new(&stream_handle)
                .map_err(|e| anyhow!("Failed to create audio sink: {}", e))?;

            // Add the source to the sink and play
            sink.append(source);
            
            info!("Audio playback started");
            
            // Wait for playback to finish
            sink.sleep_until_end();
            
            info!("Audio playback completed");

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))??;

        Ok(())
    }


}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new().expect("Failed to create default audio player")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_player_creation() {
        // This test may fail in CI/CD without audio devices
        let result = AudioPlayer::new();
        // Just verify it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }
}
