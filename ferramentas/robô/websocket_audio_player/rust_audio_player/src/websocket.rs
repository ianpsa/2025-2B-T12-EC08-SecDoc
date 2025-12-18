use bytes::Bytes;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct AudioMessage {
    pub audio: String,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "mp3".to_string()
}

/// Zero-copy message for internal processing
pub struct AudioData {
    pub audio_b64: String,
    pub format: String,
}

pub async fn connect_and_receive(websocket_url: &str, tx: mpsc::Sender<AudioData>) {
    let url = match Url::parse(websocket_url) {
        Ok(u) => u,
        Err(_) => return,
    };

    loop {
        println!("🔌 Connecting to {}...", websocket_url);

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                println!("✅ Connected!\n");
                let (_, mut read) = ws_stream.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            // Fast JSON parse
                            if let Ok(audio_msg) = serde_json::from_str::<AudioMessage>(&text) {
                                let data = AudioData {
                                    audio_b64: audio_msg.audio,
                                    format: audio_msg.format,
                                };
                                if tx.send(data).await.is_err() {
                                    return; // Receiver dropped
                                }
                            }
                        }
                        Ok(Message::Binary(bin)) => {
                            // Support raw binary format: first byte = format length, 
                            // next N bytes = format string, rest = audio data
                            if let Some(data) = parse_binary_message(&bin) {
                                if tx.send(data).await.is_err() {
                                    return;
                                }
                            }
                        }
                        Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                        Ok(Message::Close(_)) => break,
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Connection failed: {}", e);
            }
        }

        println!("🔄 Reconnecting in 1s...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Parse binary message format (more efficient than JSON+base64)
fn parse_binary_message(data: &[u8]) -> Option<AudioData> {
    if data.len() < 2 {
        return None;
    }
    
    let format_len = data[0] as usize;
    if data.len() < 1 + format_len {
        return None;
    }
    
    let format = String::from_utf8_lossy(&data[1..1 + format_len]).to_string();
    let audio_bytes = &data[1 + format_len..];
    
    // Convert to base64 for compatibility with existing pipeline
    let audio_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        audio_bytes
    );
    
    Some(AudioData { audio_b64, format })
}
