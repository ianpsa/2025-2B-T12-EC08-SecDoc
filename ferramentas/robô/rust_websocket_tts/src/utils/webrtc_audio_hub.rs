use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::sync::Arc;
use crate::utils::webrtc_connection::UnitreeWebRTCConnection;

// Audio API IDs matching Python implementation
pub const AUDIO_API_GET_AUDIO_LIST: u32 = 1001;
pub const AUDIO_API_SELECT_START_PLAY: u32 = 1002;
pub const AUDIO_API_UPLOAD_AUDIO_FILE: u32 = 2001;

// WebRTC data channel topic
pub const AUDIO_HUB_REQUEST_TOPIC: &str = "rt/api/audiohub/request";

#[derive(Debug, Serialize, Deserialize)]
struct AudioUploadChunk {
    file_name: String,
    file_type: String,
    file_size: usize,
    current_block_index: usize,
    total_block_number: usize,
    block_content: String,
    current_block_size: usize,
    file_md5: String,
    create_time: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayByUuidParameter {
    unique_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioListResponse {
    pub data: Vec<AudioFileInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioFileInfo {
    pub unique_id: String,
    pub file_name: String,
}

/// WebRTC Audio Hub for Unitree Go2 robot
/// Handles audio upload and playback via WebRTC data channel
pub struct WebRTCAudioHub {
    connection: Arc<UnitreeWebRTCConnection>,
}

impl WebRTCAudioHub {
    pub fn new(robot_ip: String) -> Self {
        Self {
            connection: Arc::new(UnitreeWebRTCConnection::new(robot_ip)),
        }
    }

    /// Initialize the WebRTC connection to the robot
    pub async fn connect(&self) -> Result<(), anyhow::Error> {
        self.connection.connect().await
    }

    /// Upload WAV audio file in chunks via WebRTC data channel
    /// Returns the MD5 hash of the uploaded file
    pub async fn upload_audio_wav(
        &self,
        file_name: &str,
        wav_data: Vec<u8>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        println!("[WEBRTC] Preparing to upload audio file: {}", file_name);
        println!("[WEBRTC] Audio size: {} bytes", wav_data.len());

        // Calculate MD5 hash
        let file_md5 = format!("{:x}", md5::compute(&wav_data));
        println!("[WEBRTC] File MD5: {}", file_md5);

        // Convert to base64
        let b64_data = general_purpose::STANDARD.encode(&wav_data);
        println!("[WEBRTC] Base64 encoded size: {} bytes", b64_data.len());

        // Split into 4KB chunks (matching Python implementation)
        const CHUNK_SIZE: usize = 4096;
        let chunks: Vec<&str> = b64_data
            .as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect();
        
        let total_chunks = chunks.len();
        println!("[WEBRTC] Splitting file into {} chunks of max {}KB each", total_chunks, CHUNK_SIZE / 1024);

        // Get current timestamp
        let create_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;

        // Send each chunk
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_index = i + 1;
            
            let chunk_data = AudioUploadChunk {
                file_name: file_name.to_string(),
                file_type: "wav".to_string(),
                file_size: wav_data.len(),
                current_block_index: chunk_index,
                total_block_number: total_chunks,
                block_content: chunk.to_string(),
                current_block_size: chunk.len(),
                file_md5: file_md5.clone(),
                create_time,
            };

            let parameter = serde_json::to_string(&chunk_data)?;

            let request_data = json!({
                "api_id": AUDIO_API_UPLOAD_AUDIO_FILE,
                "parameter": parameter
            });

            println!("[WEBRTC] Sending chunk {}/{}", chunk_index, total_chunks);
            
            // Send request via WebRTC data channel
            self.connection
                .publish_request(AUDIO_HUB_REQUEST_TOPIC, request_data)
                .await
                .map_err(|e| format!("Failed to send chunk: {}", e))?;

            // Small delay between chunks to avoid overwhelming the connection
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        println!("[WEBRTC] ✓ Audio upload complete");
        Ok(file_md5)
    }

    /// Get list of audio files available on the robot
    pub async fn get_audio_list(&self) -> Result<AudioListResponse, Box<dyn Error + Send + Sync>> {
        println!("[WEBRTC] Fetching audio list from robot...");
        
        let request_data = json!({
            "api_id": AUDIO_API_GET_AUDIO_LIST,
            "parameter": "{}"
        });

        // Send request and parse response
        let response = self.connection
            .publish_request(AUDIO_HUB_REQUEST_TOPIC, request_data)
            .await
            .map_err(|e| format!("Failed to get audio list: {}", e))?;
        
        let audio_list: AudioListResponse = serde_json::from_value(response.data)?;
        
        println!("[WEBRTC] Found {} audio files", audio_list.data.len());
        Ok(audio_list)
    }

    /// Play audio by UUID
    pub async fn play_by_uuid(&self, uuid: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("[WEBRTC] Playing audio with UUID: {}", uuid);
        
        let param = PlayByUuidParameter {
            unique_id: uuid.to_string(),
        };

        let parameter = serde_json::to_string(&param)?;

        let request_data = json!({
            "api_id": AUDIO_API_SELECT_START_PLAY,
            "parameter": parameter
        });

        self.connection
            .publish_request(AUDIO_HUB_REQUEST_TOPIC, request_data)
            .await
            .map_err(|e| format!("Failed to play audio: {}", e))?;
        
        println!("[WEBRTC] ✓ Playback command sent");
        Ok(())
    }
}
