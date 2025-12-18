use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Deserialize)]
pub struct AudioMessage {
    pub audio: String,
    #[serde(default = "default_format")]
    pub format: String,
    // Optional chunk info
    #[serde(default)]
    pub chunk_index: Option<usize>,
    #[serde(default)]
    pub total_chunks: Option<usize>,
}

fn default_format() -> String {
    "mp3".to_string()
}

/// Complete audio ready for playback
pub struct AudioData {
    pub audio_b64: String,
    pub format: String,
}

/// Accumulator for chunked audio
struct ChunkAccumulator {
    chunks: Vec<Option<String>>,
    format: String,
    total: usize,
    received: usize,
}

impl ChunkAccumulator {
    fn new(total: usize, format: String) -> Self {
        Self {
            chunks: vec![None; total],
            format,
            total,
            received: 0,
        }
    }

    fn add(&mut self, index: usize, data: String) {
        if index < self.total && self.chunks[index].is_none() {
            self.chunks[index] = Some(data);
            self.received += 1;
        }
    }

    fn progress(&self) -> f32 {
        self.received as f32 / self.total as f32
    }

    /// Check if we have enough continuous data from the start
    fn has_playable_prefix(&self, min_ratio: f32) -> bool {
        let needed = ((self.total as f32) * min_ratio).ceil() as usize;
        self.chunks.iter().take(needed).all(|c| c.is_some())
    }

    fn is_complete(&self) -> bool {
        self.received == self.total
    }

    /// Combine all received chunks into single base64 string
    fn combine(&self) -> Option<String> {
        let mut result = String::new();
        for chunk in &self.chunks {
            match chunk {
                Some(data) => result.push_str(data),
                None => break, // Stop at first missing chunk
            }
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

pub async fn connect_and_receive(websocket_url: &str, tx: mpsc::Sender<AudioData>) {
    loop {
        println!("🔌 Connecting to {}...", websocket_url);

        match connect_async(websocket_url).await {
            Ok((ws_stream, _)) => {
                println!("✅ Connected!\n");
                let (_, mut read) = ws_stream.split();

                let mut accumulator: Option<ChunkAccumulator> = None;
                let mut played_partial = false;

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(audio_msg) = serde_json::from_str::<AudioMessage>(&text) {
                                // Check if it's a chunked message
                                match (audio_msg.chunk_index, audio_msg.total_chunks) {
                                    (Some(idx), Some(total)) if total > 1 => {
                                        // Chunked mode
                                        println!("📥 Chunk {}/{}", idx + 1, total);
                                        
                                        // Initialize accumulator if needed
                                        if accumulator.is_none() {
                                            accumulator = Some(ChunkAccumulator::new(
                                                total,
                                                audio_msg.format.clone(),
                                            ));
                                            played_partial = false;
                                        }

                                        if let Some(ref mut acc) = accumulator {
                                            acc.add(idx, audio_msg.audio);
                                            
                                            // Play when we have 50% (only once)
                                            if !played_partial && acc.has_playable_prefix(0.5) {
                                                println!("🎵 50% received, starting playback...");
                                                if let Some(combined) = acc.combine() {
                                                    let data = AudioData {
                                                        audio_b64: combined,
                                                        format: acc.format.clone(),
                                                    };
                                                    if tx.send(data).await.is_err() {
                                                        return;
                                                    }
                                                    played_partial = true;
                                                }
                                            }
                                            
                                            // Send complete audio when done
                                            if acc.is_complete() {
                                                println!("✅ All chunks received");
                                                if let Some(combined) = acc.combine() {
                                                    // Only send if we haven't played yet, or send full version
                                                    if !played_partial {
                                                        let data = AudioData {
                                                            audio_b64: combined,
                                                            format: acc.format.clone(),
                                                        };
                                                        let _ = tx.send(data).await;
                                                    }
                                                }
                                                accumulator = None;
                                            }
                                        }
                                    }
                                    _ => {
                                        // Single message - send immediately
                                        println!("📥 Complete audio received");
                                        let data = AudioData {
                                            audio_b64: audio_msg.audio,
                                            format: audio_msg.format,
                                        };
                                        if tx.send(data).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                        }
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
