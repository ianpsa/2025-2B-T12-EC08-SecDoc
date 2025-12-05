use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, error, warn};

use crate::audio_decoder::AudioPlayer;

/// Response from the backend model
#[derive(Debug, Deserialize, Serialize)]
pub struct ModelResponse {
    /// Response message
    pub message: Option<String>,
    /// Response data with answer details
    pub data: Option<RespostaData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RespostaData {
    pub id: Option<i32>,
    pub pergunta_id: Option<i32>,
    pub respondido_por_tipo: Option<String>,
    pub respondido_por_usuario: Option<i32>,
    pub texto: String,
    pub criado_em: Option<String>,
}

/// Done signal from server
#[derive(Debug, Deserialize, Serialize)]
pub struct DoneSignal {
    pub done: bool,
}

/// Error response from server
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// WebSocket client that receives and plays audio
pub struct WebSocketAudioClient {
    url: String,
    audio_player: AudioPlayer,
}

impl WebSocketAudioClient {
    /// Create a new WebSocket audio client
    pub fn new(url: String) -> Result<Self> {
        let audio_player = AudioPlayer::new()?;
        
        Ok(Self {
            url,
            audio_player,
        })
    }

    /// Connect to WebSocket server and listen for audio messages
    /// Returns when connection is closed or encounters an error
    pub async fn connect_and_listen(&self) -> Result<()> {
        info!("Connecting to WebSocket server: {}", self.url);
        
        // Connect to the WebSocket server
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .context("Failed to connect to WebSocket server")?;

        info!("Connected successfully!");

        let (_write, mut read) = ws_stream.split();

        // State tracking for multi-message responses
        let mut current_text_response: Option<String> = None;

        // Listen for messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Received text message: {} bytes", text.len());
                    
                    // Check if it's a done signal
                    if let Ok(done) = serde_json::from_str::<DoneSignal>(&text) {
                        if done.done {
                            info!("✓ Processing complete signal received");
                            current_text_response = None;
                            continue;
                        }
                    }
                    
                    // Check if it's an error
                    if let Ok(error) = serde_json::from_str::<ErrorResponse>(&text) {
                        error!("✗ Server error: {}", error.error);
                        current_text_response = None;
                        continue;
                    }
                    
                    // Try to parse as model response
                    if let Ok(response) = serde_json::from_str::<ModelResponse>(&text) {
                        if let Some(data) = &response.data {
                            info!("✓ Text response received: {}", data.texto);
                            current_text_response = Some(data.texto.clone());
                        } else if let Some(msg) = &response.message {
                            info!("✓ Message: {}", msg);
                        }
                    } else {
                        // Unknown text message format
                        warn!("Received unrecognized text message: {}", text);
                    }
                }
                Ok(Message::Binary(data)) => {
                    info!("Received binary audio message: {} bytes", data.len());
                    
                    // Binary data is raw audio (not base64-encoded)
                    // Play it directly
                    if let Some(text) = &current_text_response {
                        info!("Playing audio response for: \"{}\"", text);
                    }
                    
                    self.process_binary_audio(&data).await;
                }
                Ok(Message::Ping(_)) => {
                    // Pong is sent automatically by the library
                }
                Ok(Message::Pong(_)) => {
                    // Ignore pong messages
                }
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        info!("Server closed connection: {} - {}", frame.code, frame.reason);
                    } else {
                        info!("Server closed connection");
                    }
                    break;
                }
                Ok(Message::Frame(_)) => {
                    // Raw frames are handled by the library
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        info!("Connection closed");
        Ok(())
    }

    /// Process binary audio data (raw audio, not base64)
    async fn process_binary_audio(&self, audio_bytes: &[u8]) {
        info!("Processing raw binary audio: {} bytes", audio_bytes.len());

        if audio_bytes.is_empty() {
            error!("Received empty audio data");
            return;
        }

        // The backend sends raw audio data (MP3/WAV/OGG), not base64-encoded
        // Try to detect format based on magic bytes
        let format = Self::detect_audio_format(audio_bytes);
        info!("Detected audio format: {}", format);

        // Play the audio
        match self.audio_player.play_audio(audio_bytes, &format).await {
            Ok(_) => {
                info!("✓ Audio playback completed successfully");
            }
            Err(e) => {
                error!("✗ Failed to play audio: {}", e);
            }
        }
    }

    /// Detect audio format from magic bytes
    fn detect_audio_format(data: &[u8]) -> String {
        if data.len() < 4 {
            return "mp3".to_string(); // Default fallback
        }

        // Check for common audio file signatures
        match &data[..4] {
            // MP3: ID3 tag or MPEG frame sync
            [0x49, 0x44, 0x33, ..] => "mp3".to_string(), // ID3
            [0xFF, 0xFB, ..] => "mp3".to_string(), // MPEG-1 Layer 3
            [0xFF, 0xF3, ..] => "mp3".to_string(), // MPEG-1 Layer 3
            [0xFF, 0xF2, ..] => "mp3".to_string(), // MPEG-2 Layer 3
            
            // WAV: RIFF header
            [0x52, 0x49, 0x46, 0x46] => "wav".to_string(), // RIFF
            
            // OGG: OggS header
            [0x4F, 0x67, 0x67, 0x53] => "ogg".to_string(), // OggS
            
            // FLAC: fLaC header
            [0x66, 0x4C, 0x61, 0x43] => "flac".to_string(), // fLaC
            
            _ => {
                warn!("Unknown audio format, assuming MP3");
                "mp3".to_string()
            }
        }
    }
}
