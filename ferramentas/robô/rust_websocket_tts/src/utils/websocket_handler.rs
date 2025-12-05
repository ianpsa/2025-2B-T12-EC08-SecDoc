use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, error};

use crate::audio_decoder::AudioPlayer;

/// Message format expected from WebSocket server
#[derive(Debug, Deserialize, Serialize)]
pub struct AudioMessage {
    /// Base64 encoded audio data
    pub audio_data: String,
    /// Audio format (e.g., "mp3", "wav", "ogg")
    #[serde(default = "default_format")]
    pub format: String,
    /// Optional message ID for tracking
    #[serde(default)]
    pub message_id: Option<String>,
}

fn default_format() -> String {
    "mp3".to_string()
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

        // Listen for messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Received text message: {} bytes", text.len());
                    self.process_audio_message(&text).await;
                }
                Ok(Message::Binary(data)) => {
                    info!("Received binary message: {} bytes", data.len());
                    
                    // Try to decode as JSON first
                    if let Ok(text) = String::from_utf8(data) {
                        self.process_audio_message(&text).await;
                    } else {
                        error!("Binary message is not valid UTF-8");
                    }
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

    /// Process an audio message and play it
    async fn process_audio_message(&self, text: &str) {
        // Parse the JSON message
        let audio_msg: AudioMessage = match serde_json::from_str(text) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to parse JSON: {}", e);
                error!("Message: {}", text);
                return;
            }
        };

        info!(
            "Processing audio message - Format: {}{}",
            audio_msg.format,
            audio_msg.message_id.as_ref().map(|id| format!(", ID: {}", id)).unwrap_or_default()
        );

        // Decode base64 audio data
        let audio_bytes = match base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &audio_msg.audio_data
        ) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to decode base64: {}", e);
                return;
            }
        };

        info!("Decoded {} bytes of audio data", audio_bytes.len());

        // Play the audio
        match self.audio_player.play_audio(&audio_bytes, &audio_msg.format).await {
            Ok(_) => {
                info!("Audio playback completed successfully");
            }
            Err(e) => {
                error!("Failed to play audio: {}", e);
            }
        }
    }
}
