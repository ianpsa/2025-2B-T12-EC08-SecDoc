use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use webrtc::media::Sample;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use std::time::Duration;

/// Audio player that streams PCM audio directly to robot via WebRTC audio track
/// Similar to Python's MediaPlayer approach in play_mp3.py
pub struct WebRTCAudioPlayer {
    audio_track: Arc<TrackLocalStaticSample>,
    audio_queue: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    is_playing: Arc<Mutex<bool>>,
}

impl WebRTCAudioPlayer {
    /// Create a new audio player with an audio track
    pub fn new(audio_receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        // Create audio track with Opus codec (standard for WebRTC audio)
        // Opus parameters: 48kHz clock rate, 2 channels (stereo)
        let audio_track = Arc::new(TrackLocalStaticSample::new(
            webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability {
                mime_type: "audio/opus".to_owned(),
                clock_rate: 48000,  // Opus uses 48kHz clock rate
                channels: 2,        // Stereo
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            "audio".to_owned(),
            "webrtc-audio-player".to_owned(),
        ));

        Self {
            audio_track,
            audio_queue: Arc::new(Mutex::new(audio_receiver)),
            is_playing: Arc::new(Mutex::new(false)),
        }
    }

    /// Get the audio track to add to peer connection
    pub fn get_track(&self) -> Arc<TrackLocalStaticSample> {
        self.audio_track.clone()
    }

    /// Start playing audio from the queue
    /// This spawns a background task that reads from the audio queue and streams to WebRTC
    pub async fn start_playback(&self) {
        let mut is_playing = self.is_playing.lock().await;
        if *is_playing {
            println!("[AUDIO_PLAYER] Already playing");
            return;
        }
        *is_playing = true;
        drop(is_playing);

        let audio_track = self.audio_track.clone();
        let audio_queue = self.audio_queue.clone();
        let is_playing_flag = self.is_playing.clone();

        tokio::spawn(async move {
            println!("[AUDIO_PLAYER] Starting audio playback task");
            let mut receiver = audio_queue.lock().await;
            
            while let Some(pcm_data) = receiver.recv().await {
                println!("[AUDIO_PLAYER] Received {} bytes of PCM data", pcm_data.len());
                
                // Convert PCM to proper format and stream
                // Assuming 16kHz mono 16-bit PCM input (from audio_decoder)
                // WebRTC expects samples at regular intervals
                
                // Split into frames (20ms chunks for opus)
                // 16kHz * 0.02s = 320 samples = 640 bytes (16-bit)
                const FRAME_SIZE: usize = 640; // 20ms at 16kHz mono 16-bit
                const SAMPLE_DURATION: Duration = Duration::from_millis(20);
                
                let chunks: Vec<_> = pcm_data.chunks(FRAME_SIZE).collect();
                println!("[AUDIO_PLAYER] Streaming {} frames", chunks.len());
                
                for (i, chunk) in chunks.iter().enumerate() {
                    let sample = Sample {
                        data: chunk.to_vec().into(),
                        duration: SAMPLE_DURATION,
                        ..Default::default()
                    };
                    
                    if let Err(e) = audio_track.write_sample(&sample).await {
                        eprintln!("[AUDIO_PLAYER] Error writing sample {}: {}", i, e);
                        break;
                    }
                    
                    // Pace the streaming to match real-time playback
                    tokio::time::sleep(SAMPLE_DURATION).await;
                }
                
                println!("[AUDIO_PLAYER] Finished streaming audio chunk");
            }
            
            println!("[AUDIO_PLAYER] Audio queue closed, stopping playback");
            let mut is_playing = is_playing_flag.lock().await;
            *is_playing = false;
        });
    }
}
