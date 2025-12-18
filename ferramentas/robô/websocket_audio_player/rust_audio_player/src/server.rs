use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::audio::AudioProcessor;
use crate::player::RobotPlayer;

#[derive(Debug, Clone, Deserialize)]
struct AudioItem {
    id: String,
}

#[derive(Debug, Deserialize)]
struct Checkpoints {
    #[serde(flatten)]
    checkpoints: HashMap<String, Vec<AudioItem>>,
}

#[derive(Clone)]
struct AppState {
    player: Arc<RobotPlayer>,
    processor: Arc<AudioProcessor>,
    audio_dir: PathBuf,
    checkpoints: Arc<Checkpoints>,
    playing: Arc<Mutex<Option<String>>>, // Current checkpoint being played
}

#[derive(Serialize)]
struct Response {
    status: String,
    message: String,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    playing: Option<String>,
    checkpoints: HashMap<String, usize>,
}

pub async fn start_server(
    port: u16,
    player: Arc<RobotPlayer>,
    processor: Arc<AudioProcessor>,
    audio_dir: PathBuf,
) {
    // Load checkpoints.json
    let checkpoints_path = std::env::current_dir()
        .unwrap_or_default()
        .join("checkpoints.json");
    
    println!("📂 Loading checkpoints from: {:?}", checkpoints_path);
    
    let checkpoints: Checkpoints = match std::fs::read_to_string(&checkpoints_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("❌ Failed to parse checkpoints.json: {}", e);
            Checkpoints {
                checkpoints: HashMap::new(),
            }
        }),
        Err(e) => {
            eprintln!("❌ Failed to read checkpoints.json: {}", e);
            Checkpoints {
                checkpoints: HashMap::new(),
            }
        }
    };

    println!("📋 Loaded {} checkpoints", checkpoints.checkpoints.len());
    for (name, items) in &checkpoints.checkpoints {
        println!("   {} -> {} audios", name, items.len());
    }
    
    println!("📂 Audio directory: {:?}", audio_dir);

    let state = AppState {
        player,
        processor,
        audio_dir,
        checkpoints: Arc::new(checkpoints),
        playing: Arc::new(Mutex::new(None)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(status))
        .route("/checkpoints", get(list_checkpoints))
        .route("/:checkpoint", post(play_checkpoint))
        .route("/stop", post(stop_playback))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 HTTP server ready on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Json<Response> {
    Json(Response {
        status: "ok".to_string(),
        message: "Audio Player API. POST /:checkpoint to play, GET /status for info".to_string(),
    })
}

async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let playing = state.playing.lock().await.clone();
    let mut checkpoints = HashMap::new();
    for (name, items) in &state.checkpoints.checkpoints {
        checkpoints.insert(name.clone(), items.len());
    }
    Json(StatusResponse {
        status: "ok".to_string(),
        playing,
        checkpoints,
    })
}

async fn list_checkpoints(State(state): State<AppState>) -> Json<HashMap<String, usize>> {
    let mut result = HashMap::new();
    for (name, items) in &state.checkpoints.checkpoints {
        result.insert(name.clone(), items.len());
    }
    Json(result)
}

async fn play_checkpoint(
    State(state): State<AppState>,
    Path(checkpoint): Path<String>,
) -> Result<Json<Response>, (StatusCode, Json<Response>)> {
    println!("\n📥 Received request for checkpoint: {}", checkpoint);
    
    // Check if already playing
    {
        let playing = state.playing.lock().await;
        if playing.is_some() {
            return Err((
                StatusCode::CONFLICT,
                Json(Response {
                    status: "error".to_string(),
                    message: format!("Already playing: {}", playing.as_ref().unwrap()),
                }),
            ));
        }
    }

    // Get checkpoint audio list
    let audio_list = match state.checkpoints.checkpoints.get(&checkpoint) {
        Some(list) => list.clone(),
        None => {
            println!("❌ Checkpoint not found: {}", checkpoint);
            return Err((
                StatusCode::NOT_FOUND,
                Json(Response {
                    status: "error".to_string(),
                    message: format!("Checkpoint '{}' not found. Available: {:?}", 
                        checkpoint, 
                        state.checkpoints.checkpoints.keys().collect::<Vec<_>>()),
                }),
            ));
        }
    };

    println!("🎵 Starting checkpoint: {} ({} audios)", checkpoint, audio_list.len());

    // Mark as playing
    {
        let mut playing = state.playing.lock().await;
        *playing = Some(checkpoint.clone());
    }

    // Play in background and return immediately
    let state_clone = state.clone();
    let checkpoint_clone = checkpoint.clone();
    
    tokio::spawn(async move {
        play_audio_list(state_clone, checkpoint_clone, audio_list).await;
    });

    Ok(Json(Response {
        status: "started".to_string(),
        message: format!("Playing checkpoint: {}", checkpoint),
    }))
}

async fn play_audio_list(state: AppState, checkpoint: String, audio_list: Vec<AudioItem>) {
    let mut played = 0;
    let total = audio_list.len();

    for item in &audio_list {
        // Check for stop signal
        {
            let playing = state.playing.lock().await;
            if playing.is_none() {
                println!("⏹️ Playback stopped");
                break;
            }
        }

        // Find audio file
        let audio_path = find_audio_file(&state.audio_dir, &item.id);
        
        match audio_path {
            Some(path) => {
                println!("   ▶️ [{}/{}] Playing: {}", played + 1, total, item.id);
                
                // Check if already WAV - skip decode
                let is_wav = path.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("wav"))
                    .unwrap_or(false);

                let wav_path = if is_wav {
                    // Already WAV, use directly
                    path.clone()
                } else {
                    // Need to decode
                    let audio_data = match tokio::fs::read(&path).await {
                        Ok(data) => data,
                        Err(e) => {
                            println!("   ❌ Read error: {} - {}", item.id, e);
                            continue;
                        }
                    };

                    let format = path.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("mp3");

                    let b64 = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &audio_data,
                    );

                    match state.processor.decode(&b64, format).await {
                        Some(p) => p,
                        None => {
                            println!("   ❌ Decode failed: {}", item.id);
                            continue;
                        }
                    }
                };

                // Send to robot
                let success = state.player.send_audio(&wav_path).await;
                
                if success {
                    played += 1;
                    println!("   ✅ Done: {}", item.id);
                } else {
                    println!("   ⚠️ Player returned false: {}", item.id);
                }

                // Cleanup decoded file (not original)
                if !is_wav {
                    let proc = state.processor.clone();
                    let wav_clone = wav_path.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        proc.cleanup(&wav_clone);
                    });
                }
            }
            None => {
                println!("   ⚠️ Not found: {}", item.id);
            }
        }
    }

    // Mark as not playing
    {
        let mut playing = state.playing.lock().await;
        *playing = None;
    }

    println!("✅ Checkpoint {} complete: {}/{} played", checkpoint, played, total);
}

async fn stop_playback(State(state): State<AppState>) -> Json<Response> {
    let mut playing = state.playing.lock().await;
    let was_playing = playing.take();
    Json(Response {
        status: "success".to_string(),
        message: match was_playing {
            Some(cp) => format!("Stopped: {}", cp),
            None => "Nothing was playing".to_string(),
        },
    })
}

fn find_audio_file(audio_dir: &PathBuf, id: &str) -> Option<PathBuf> {
    // First try WAV (preferred - no decode needed)
    let extensions = ["wav", "mp3", "ogg", "m4a", "flac"];
    
    for ext in &extensions {
        let path = audio_dir.join(format!("{}.{}", id, ext));
        if path.exists() {
            return Some(path);
        }
    }
    
    // Try without extension
    let path = audio_dir.join(id);
    if path.exists() {
        return Some(path);
    }

    None
}
