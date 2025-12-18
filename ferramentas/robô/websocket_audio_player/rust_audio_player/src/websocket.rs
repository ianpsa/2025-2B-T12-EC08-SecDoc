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

pub async fn connect_and_receive(
    websocket_url: &str,
    tx: mpsc::Sender<AudioMessage>,
) {
    let url = match Url::parse(websocket_url) {
        Ok(u) => u,
        Err(_) => return,
    };

    loop {
        println!(" Connecting...");

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                println!("✅ Connected\n");
                let (_, mut read) = ws_stream.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(audio_msg) = serde_json::from_str::<AudioMessage>(&text) {
                                if tx.send(audio_msg).await.is_err() {
                                    return;
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                eprintln!(" Connection failed: {}", e);
            }
        }

        println!(" Reconnecting in 2s...");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
