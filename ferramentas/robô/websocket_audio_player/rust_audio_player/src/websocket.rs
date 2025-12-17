use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};
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

pub async fn connect_and_receive(
    websocket_url: &str,
    tx: mpsc::Sender<AudioMessage>,
) {
    let url = match Url::parse(websocket_url) {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid WebSocket URL: {}", e);
            return;
        }
    };

    loop {
        info!(" Connecting to {}...", websocket_url);

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                info!(" WebSocket connected");
                let (_, mut read) = ws_stream.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(audio_msg) = serde_json::from_str::<AudioMessage>(&text) {
                                info!(" Received audio ({} format)", audio_msg.format);
                                if tx.send(audio_msg).await.is_err() {
                                    error!("Channel closed");
                                    return;
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Connection failed: {}", e);
            }
        }

        info!("Reconnecting in 3 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

