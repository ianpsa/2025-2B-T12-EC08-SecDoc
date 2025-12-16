use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
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
) {
    // Remove Result return type to avoid propagating errors that close the connection
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during WebSocket handshake: {}", e);
            return;
        }
    };
    
    let (mut write, mut read) = ws_stream.split();
    
    println!("New WebSocket connection established");
    
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received text message ({} chars)", text.len());
                
                // Decode base64 to audio bytes
                use base64::Engine;
                match base64::engine::general_purpose::STANDARD.decode(&text) {
                    Ok(audio_data) => {
                        println!("Decoded {} bytes from base64", audio_data.len());
                        
                        // Send to processing pipeline
                        if let Err(e) = audio_sender.send(audio_data).await {
                            eprintln!("Failed to send audio data to pipeline: {}", e);
                            // Send error message back to client
                            let _ = write.send(Message::Text(format!("ERROR: Failed to process audio: {}", e))).await;
                            break;
                        }
                        
                        // Send acknowledgment
                        let _ = write.send(Message::Text("OK: Audio received".to_string())).await;
                    }
                    Err(e) => {
                        eprintln!("Failed to decode base64: {}", e);
                        // Send error but don't close connection
                        let _ = write.send(Message::Text(format!("ERROR: Invalid base64: {}", e))).await;
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                println!("Received binary message ({} bytes)", data.len());
                
                // Send raw binary data to processing pipeline
                if let Err(e) = audio_sender.send(data.to_vec()).await {
                    eprintln!("Failed to send audio data to pipeline: {}", e);
                    let _ = write.send(Message::Text(format!("ERROR: Failed to process audio: {}", e))).await;
                    break;
                }
                
                // Send acknowledgment
                let _ = write.send(Message::Text("OK: Audio received".to_string())).await;
            }
            Ok(Message::Close(_)) => {
                println!("WebSocket connection closed by client");
                break;
            }
            Ok(Message::Ping(payload)) => {
                let _ = write.send(Message::Pong(payload)).await;
            }
            Ok(_) => {
                // Ignore other message types
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
        }
    }
    
    println!("Connection handler terminated");
}
