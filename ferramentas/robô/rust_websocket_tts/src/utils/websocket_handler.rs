use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, error, warn};

// Import base64 for decoding base64-encoded audio
use base64::Engine as _;
use base64::engine::general_purpose;

// Import our new modules
use crate::mp3_decoder::decode_mp3_to_pcm;
use crate::utils::ros_audio_publisher::RosAudioPublisher;

/// Message to send to backend - Text input
#[derive(Debug, Serialize)]
pub struct TextRequest {
    #[serde(rename = "type")]
    pub message_type: String,
    pub texto: String,
    pub checkpoint_id: i32,
    pub estado: String,
    pub liberado_em: Option<String>,
    pub question_topic: Option<String>,
    pub respondido_em: Option<String>,
    pub tour_id: Option<i32>,
}

impl TextRequest {
    pub fn new(text: String) -> Self {
        Self {
            message_type: "text".to_string(),
            texto: text,
            checkpoint_id: 1,
            estado: "pendente".to_string(),
            liberado_em: None,
            question_topic: Some("general".to_string()),
            respondido_em: None,
            tour_id: Some(1),
        }
    }
}

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

/// WebSocket client that receives audio and publishes to ROS2
/// 
/// Flow: WebSocket → Base64 decode (if needed) → MP3 decode → ROS2 publish → Robot plays
pub struct WebSocketAudioClient {
    url: String,
    /// ROS2 publisher for sending audio to the Unitree GO2
    ros_publisher: RosAudioPublisher,
}

impl WebSocketAudioClient {
    /// Create a new WebSocket audio client with ROS2 publisher
    /// 
    /// # Arguments
    /// * `url` - WebSocket server URL (e.g., "ws://localhost:8080/v1/audio")
    /// 
    /// # Returns
    /// * `Result<Self>` - Initialized client or error
    pub fn new(url: String) -> Result<Self> {
        // Create ROS2 audio publisher
        let ros_publisher = RosAudioPublisher::new(
            "rust_websocket_audio",  // Node name (visible in `ros2 node list`)
            "audiodata"              // Topic name
        )?;
        
        Ok(Self {
            url,
            ros_publisher,
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

        let (mut write, mut read) = ws_stream.split();
        
        // Send a test message to the backend
        info!("Sending test question to backend...");
        let test_request = TextRequest::new("Olá, como você está?".to_string());
        let message_json = serde_json::to_string(&test_request)
            .context("Failed to serialize test message")?;
        write.send(Message::Text(message_json)).await
            .context("Failed to send test message to WebSocket")?;
        info!("Test question sent, waiting for response...");

        // State tracking for multi-message responses
        let mut current_text_response: Option<String> = None;

        // Listen for messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Received text message: {} bytes", text.len());
                    
                    // ============================================================
                    // CHECK 1: Is this base64-encoded audio?
                    // ============================================================
                    // Some servers send audio as base64 text instead of binary
                    // Indicators: starts with "data:audio" or very long text (>1000 chars)
                    if text.starts_with("data:audio") || (text.len() > 1000 && !text.starts_with("{")) {
                        info!("🔍 Detected base64 audio in text message");
                        self.process_base64_audio(&text).await;
                        continue;
                    }
                    
                    // ============================================================
                    // CHECK 2: Is this a done signal?
                    // ============================================================
                    if let Ok(done) = serde_json::from_str::<DoneSignal>(&text) {
                        if done.done {
                            info!("✓ Processing complete signal received");
                            current_text_response = None;
                            continue;
                        }
                    }
                    
                    // ============================================================
                    // CHECK 3: Is this an error?
                    // ============================================================
                    if let Ok(error) = serde_json::from_str::<ErrorResponse>(&text) {
                        error!("✗ Server error: {}", error.error);
                        current_text_response = None;
                        continue;
                    }
                    
                    // ============================================================
                    // CHECK 4: Try to parse as JSON model response
                    // ============================================================
                    if let Ok(response) = serde_json::from_str::<ModelResponse>(&text) {
                        if let Some(data) = &response.data {
                            info!("✓ JSON response received: {}", data.texto);
                            current_text_response = Some(data.texto.clone());
                        } else if let Some(msg) = &response.message {
                            info!("✓ Message: {}", msg);
                            current_text_response = Some(msg.clone());
                        }
                    } else {
                        // ========================================================
                        // CHECK 5: Plain text response (not JSON)
                        // ========================================================
                        info!("✓ Text response received: {}", text);
                        current_text_response = Some(text.clone());
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

    /// Process binary audio data and publish to ROS2
    /// 
    /// This is the core audio processing pipeline:
    /// 1. Receive audio bytes (MP3 format)
    /// 2. Decode MP3 to PCM
    /// 3. Publish PCM to ROS2 for the robot to play
    async fn process_binary_audio(&self, audio_bytes: &[u8]) {
        info!("🎧 Processing audio: {} bytes", audio_bytes.len());

        if audio_bytes.is_empty() {
            error!("✗ Received empty audio data");
            return;
        }

        // ================================================================
        // STEP 1: Detect audio format (we expect MP3)
        // ================================================================
        let format = Self::detect_audio_format(audio_bytes);
        info!("📊 Detected audio format: {}", format);

        // ================================================================
        // STEP 2: Decode MP3 to PCM
        // ================================================================
        // Only MP3 is supported currently (add other formats later if needed)
        if format != "mp3" {
            warn!("⚠️  Non-MP3 format detected: {}. Trying MP3 decode anyway...", format);
        }

        // Decode MP3 to PCM
        let decoded = match decode_mp3_to_pcm(audio_bytes) {
            Ok(d) => d,
            Err(e) => {
                error!("✗ MP3 decode failed: {}", e);
                return;
            }
        };

        info!("✓ Decoded audio: {} bytes PCM, {}Hz, {} channels", 
              decoded.samples.len(), 
              decoded.sample_rate, 
              decoded.channels);

        // Publish PCM audio to ROS2
        match self.ros_publisher.publish_audio(decoded.samples).await {
            Ok(_) => {
                info!("🎉 Audio published to ROS2 successfully!");
            }
            Err(e) => {
                error!("✗ Failed to publish to ROS2: {}", e);
            }
        }
    }
    
    /// Process base64-encoded audio (if server sends as text)
    /// 
    /// Some servers send audio as base64-encoded text instead of binary.
    /// This handles that case: base64 text → raw bytes → MP3 decode → ROS2
    async fn process_base64_audio(&self, base64_text: &str) {
        info!("🔓 Decoding base64 audio: {} chars", base64_text.len());
        
        // Extract base64 data (skip "data:audio/mp3;base64," if present)
        let base64_data = if base64_text.contains(";base64,") {
            let parts: Vec<&str> = base64_text.split(";base64,").collect();
            if parts.len() >= 2 {
                parts[1]
            } else {
                base64_text
            }
        } else {
            base64_text
        };
        
        // Decode base64 to raw bytes
        let audio_bytes = match general_purpose::STANDARD.decode(base64_data) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("✗ Base64 decode failed: {}", e);
                return;
            }
        };
        
        info!("✓ Decoded base64 to {} bytes", audio_bytes.len());
        
        // Process as binary audio (MP3 decode → ROS2 publish)
        self.process_binary_audio(&audio_bytes).await;
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
