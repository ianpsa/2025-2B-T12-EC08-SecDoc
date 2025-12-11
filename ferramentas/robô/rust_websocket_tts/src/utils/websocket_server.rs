use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub async fn start_websocket_server(
    addr: &str,
    audio_sender: mpsc::Sender<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    let (_write, mut read) = ws_stream.split();
    
    println!("New WebSocket connection established");
    
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                println!("Received text message ({} chars)", text.len());
                // Decode base64 to MP3 bytes
                use base64::Engine;
                let mp3_data = base64::engine::general_purpose::STANDARD.decode(text)?;
                
                println!("Decoded {} bytes from base64", mp3_data.len());
                
                // Send to processing pipeline
                if audio_sender.send(mp3_data).await.is_err() {
                    eprintln!("Failed to send audio data to pipeline");
                    break;
                }
            }
            Message::Binary(data) => {
                println!("Received binary message ({} bytes)", data.len());
                // If binary data is sent directly
                if audio_sender.send(data.to_vec()).await.is_err() {
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
