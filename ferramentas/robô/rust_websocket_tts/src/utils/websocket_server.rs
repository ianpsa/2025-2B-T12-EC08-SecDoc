use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;

pub async fn start_websocket_server(
    addr: &str,
    audio_sender: mpsc::Sender<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    println!("WebSocket server listening on: {}", addr);
    
    while let Ok((stream, _)) = listener.accept().await {
        let sender = audio_sender.clone();
        tokio::spawn(handle_connection(stream, sender));
    }
    
    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    audio_sender: mpsc::Sender<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();
    
    println!("New WebSocket connection established");
    
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                // Decode base64 to MP3 bytes
                let mp3_data = base64::decode(text)?;
                
                // Send to processing pipeline
                if audio_sender.send(mp3_data).await.is_err() {
                    eprintln!("Failed to send audio data to pipeline");
                    break;
                }
            }
            Message::Binary(data) => {
                // If binary data is sent directly (already MP3)
                if audio_sender.send(data).await.is_err() {
                    eprintln!("Failed to send audio data to pipeline");
                    break;
                }
            }
            Message::Close(_) => {
                println!("WebSocket connection closed");
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}
