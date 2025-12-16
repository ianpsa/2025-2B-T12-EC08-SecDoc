use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::TrackLocal;
use interceptor::registry::Registry;
use std::collections::HashMap;

const SIGNALING_PORT: u16 = 8081;

#[derive(Debug, Serialize, Deserialize)]
struct DataChannelRequest {
    topic: String,
    data: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataChannelResponse {
    pub topic: String,
    pub data: Value,
}

/// WebRTC connection manager for Unitree Go2
pub struct UnitreeWebRTCConnection {
    robot_ip: String,
    peer_connection: Arc<Mutex<Option<Arc<RTCPeerConnection>>>>,
    data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    response_handlers: Arc<RwLock<HashMap<String, mpsc::Sender<DataChannelResponse>>>>,
    connection_ready: Arc<Mutex<bool>>,
    ready_notifier: Arc<tokio::sync::Notify>,
}

impl UnitreeWebRTCConnection {
    pub fn new(robot_ip: String) -> Self {
        Self {
            robot_ip,
            peer_connection: Arc::new(Mutex::new(None)),
            data_channel: Arc::new(Mutex::new(None)),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            connection_ready: Arc::new(Mutex::new(false)),
            ready_notifier: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Wait for the WebRTC connection to be fully established
    pub async fn wait_until_connected(&self) -> Result<()> {
        println!("[WEBRTC] Waiting for connection to be fully established...");
        
        // Wait with timeout (30 seconds)
        let timeout = tokio::time::Duration::from_secs(30);
        
        tokio::time::timeout(timeout, async {
            loop {
                let ready = *self.connection_ready.lock().await;
                if ready {
                    break;
                }
                // Wait for notification
                self.ready_notifier.notified().await;
            }
        })
        .await
        .map_err(|_| anyhow!("Timeout waiting for WebRTC connection"))?;
        
        println!("[WEBRTC] ✓ Connection fully established and ready");
        Ok(())
    }

    /// Connect to the robot via WebRTC
    /// Audio track must be added BEFORE calling this method
    pub async fn connect(&self) -> Result<()> {
        println!("[WEBRTC] Establishing WebRTC connection to {}", self.robot_ip);

        let peer_connection = self.peer_connection.lock().await;
        let pc = peer_connection
            .as_ref()
            .ok_or_else(|| anyhow!("Peer connection not initialized. Call setup() first."))?;

        // Perform WebRTC signaling with robot
        self.perform_signaling(pc).await?;

        println!("[WEBRTC] ✓ WebRTC connection established");
        Ok(())
    }

    /// Setup the peer connection and data channel
    /// This must be called BEFORE adding tracks
    pub async fn setup(&self) -> Result<()> {
        println!("[WEBRTC] Setting up peer connection...");

        // Create a MediaEngine and register audio codecs
        let mut media_engine = MediaEngine::default();
        
        // Register Opus codec (required for audio)
        // Opus is the standard audio codec for WebRTC
        media_engine.register_default_codecs()?;
        
        println!("[WEBRTC] Registered audio codecs (Opus, PCMU, PCMA)");
        
        // Create an InterceptorRegistry
        let registry = Registry::new();
        
        // Register default interceptors
        let registry = register_default_interceptors(registry, &mut media_engine)?;

        // Create the API object with the MediaEngine
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        // Configure ICE servers
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Create peer connection
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);
        
        // Store peer connection
        *self.peer_connection.lock().await = Some(peer_connection.clone());

        // Set up connection state handler with ready notification
        let connection_ready = self.connection_ready.clone();
        let ready_notifier = self.ready_notifier.clone();
        peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let connection_ready = connection_ready.clone();
            let ready_notifier = ready_notifier.clone();
            Box::pin(async move {
                match state {
                    RTCPeerConnectionState::Connected => {
                        println!("[WEBRTC] 🟢 Connection state: Connected");
                        *connection_ready.lock().await = true;
                        ready_notifier.notify_waiters();
                    }
                    RTCPeerConnectionState::Connecting => {
                        println!("[WEBRTC] 🟡 Connection state: Connecting...");
                    }
                    RTCPeerConnectionState::Failed => {
                        println!("[WEBRTC] 🔴 Connection state: Failed");
                    }
                    RTCPeerConnectionState::Disconnected => {
                        println!("[WEBRTC] 🟠 Connection state: Disconnected");
                        *connection_ready.lock().await = false;
                    }
                    RTCPeerConnectionState::Closed => {
                        println!("[WEBRTC] ⚫ Connection state: Closed");
                        *connection_ready.lock().await = false;
                    }
                    _ => {
                        println!("[WEBRTC] Connection state changed: {:?}", state);
                    }
                }
            })
        }));

        // Create data channel for audio hub communication (if needed)
        let data_channel = peer_connection
            .create_data_channel("data", None)
            .await?;

        // Store data channel
        *self.data_channel.lock().await = Some(data_channel.clone());

        // Set up data channel handlers
        let response_handlers = self.response_handlers.clone();
        data_channel.on_open(Box::new(move || {
            println!("[WEBRTC] Data channel opened");
            Box::pin(async {})
        }));

        let response_handlers_clone = response_handlers.clone();
        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let handlers = response_handlers_clone.clone();
            Box::pin(async move {
                if let Ok(text) = String::from_utf8(msg.data.to_vec()) {
                    if let Ok(response) = serde_json::from_str::<DataChannelResponse>(&text) {
                        let handlers_lock = handlers.read().await;
                        if let Some(sender) = handlers_lock.get(&response.topic) {
                            let _ = sender.send(response).await;
                        }
                    }
                }
            })
        }));

        println!("[WEBRTC] ✓ Peer connection setup complete");
        Ok(())
    }

    /// Perform WebRTC signaling exchange with the robot
    async fn perform_signaling(&self, pc: &Arc<RTCPeerConnection>) -> Result<()> {
        println!("[WEBRTC] Starting signaling process...");

        // Create offer
        let offer = pc.create_offer(None).await?;
        pc.set_local_description(offer.clone()).await?;

        println!("[WEBRTC] Created and set local offer");
        println!("[WEBRTC] Offer SDP (first 200 chars): {}", 
            offer.sdp.chars().take(200).collect::<String>());

        // Send offer to robot and get answer
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        // Note: The correct endpoint is /offer, NOT /webrtc/offer
        // This matches the Python SDK's send_sdp_to_local_peer_old_method
        let signaling_url = format!("http://{}:{}/offer", self.robot_ip, SIGNALING_PORT);
        
        println!("[WEBRTC] Sending offer to: {}", signaling_url);
        println!("[WEBRTC] Request payload type: offer");
        
        // Payload must match Python SDK format:
        // { "id": "", "sdp": "...", "type": "offer", "token": "" }
        let payload = serde_json::json!({
            "id": "",
            "sdp": offer.sdp,
            "type": "offer",
            "token": ""
        });
        
        println!("[WEBRTC] Request payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());
        
        let start = std::time::Instant::now();
        let response = client
            .post(&signaling_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                println!("[WEBRTC] ❌ HTTP request failed: {}", e);
                anyhow!("HTTP request failed: {}", e)
            })?;

        let duration = start.elapsed();
        println!("[WEBRTC] Received response in {:?}", duration);
        
        let status = response.status();
        println!("[WEBRTC] Response status: {}", status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| "Unable to read body".to_string());
            println!("[WEBRTC] ❌ Error response body: {}", body);
            return Err(anyhow!("Signaling failed with status {}", status));
        }

        let answer: Value = response.json().await.map_err(|e| {
            println!("[WEBRTC] ❌ Failed to parse JSON response: {}", e);
            anyhow!("Failed to parse answer JSON: {}", e)
        })?;
        
        println!("[WEBRTC] Received answer JSON: {}", serde_json::to_string_pretty(&answer).unwrap_or_default());
        
        let answer_sdp = answer["sdp"]
            .as_str()
            .ok_or_else(|| {
                println!("[WEBRTC] ❌ No 'sdp' field in answer");
                anyhow!("No SDP in answer")
            })?;

        println!("[WEBRTC] Answer SDP (first 200 chars): {}", 
            answer_sdp.chars().take(200).collect::<String>());

        // Set remote description
        let remote_desc = RTCSessionDescription::answer(answer_sdp.to_string())?;
        pc.set_remote_description(remote_desc).await?;

        println!("[WEBRTC] ✓ Signaling complete - remote description set");
        Ok(())
    }

    /// Publish a request and wait for response
    pub async fn publish_request(&self, topic: &str, data: Value) -> Result<DataChannelResponse> {
        let data_channel = self.data_channel.lock().await;
        let dc = data_channel
            .as_ref()
            .ok_or_else(|| anyhow!("Data channel not connected"))?;

        // Create response channel
        let (tx, mut rx) = mpsc::channel::<DataChannelResponse>(1);
        
        // Register response handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.insert(topic.to_string(), tx);
        }

        // Send request
        let request = DataChannelRequest {
            topic: topic.to_string(),
            data,
        };
        
        let request_json = serde_json::to_string(&request)?;
        dc.send_text(request_json).await?;

        // Wait for response with timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            rx.recv()
        )
        .await
        .map_err(|_| anyhow!("Request timeout"))?
        .ok_or_else(|| anyhow!("Response channel closed"))?;

        // Clean up handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.remove(topic);
        }

        Ok(response)
    }

    /// Add an audio track to the peer connection (similar to Python's conn.pc.addTrack)
    pub async fn add_audio_track(
        &self,
        track: Arc<dyn TrackLocal + Send + Sync>,
    ) -> Result<Arc<RTCRtpSender>> {
        let peer_connection = self.peer_connection.lock().await;
        let pc = peer_connection
            .as_ref()
            .ok_or_else(|| anyhow!("Peer connection not established"))?;

        let rtp_sender = pc
            .add_track(track)
            .await
            .map_err(|e| anyhow!("Failed to add track: {}", e))?;

        println!("[WEBRTC] Audio track added to peer connection");
        Ok(rtp_sender)
    }
}
