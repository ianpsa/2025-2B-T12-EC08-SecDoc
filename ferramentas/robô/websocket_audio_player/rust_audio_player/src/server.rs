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
    playing: Arc<Mutex<bool>>,
}

#[derive(Serialize)]
struct Response {
    status: String,
    message: String,
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
    
    let checkpoints: Checkpoints = match std::fs::read_to_string(&checkpoints_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("❌ Failed to parse checkpoints.json: {}", e);
            Checkpoints {
                checkpoints: HashMap::new(),
            }
        }),
        Err(e) => {
            eprintln!("❌ Failed to read checkpoints.json: {}", e);
            eprintln!("   Looking in: {:?}", checkpoints_path);
            Checkpoints {
                checkpoints: HashMap::new(),
            }
        }
    };

    println!("📋 Loaded {} checkpoints", checkpoints.checkpoints.len());
    for (name, items) in &checkpoints.checkpoints {
        println!("   {} -> {} audios", name, items.len());
    }

    let state = AppState {
        player,
        processor,
        audio_dir,
        checkpoints: Arc::new(checkpoints),
        playing: Arc::new(Mutex::new(false)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(index))
        .route("/checkpoints", get(list_checkpoints))
        .route("/:checkpoint", post(play_checkpoint))
        .route("/stop", post(stop_playback))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 HTTP server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Json<Response> {
    Json(Response {
        status: "ok".to_string(),
        message: "Audio Player API. POST /{checkpoint_name} to play.".to_string(),
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
    // Check if already playing
    {
        let playing = state.playing.lock().await;
        if *playing {
            return Err((
                StatusCode::CONFLICT,
                Json(Response {
                    status: "error".to_string(),
                    message: "Already playing audio".to_string(),
                }),
            ));
        }
    }

    // Get checkpoint audio list
    let audio_list = match state.checkpoints.checkpoints.get(&checkpoint) {
        Some(list) => list.clone(),
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(Response {
                    status: "error".to_string(),
                    message: format!("Checkpoint '{}' not found", checkpoint),
                }),
            ));
        }
    };

    println!("\n🎵 Playing checkpoint: {} ({} audios)", checkpoint, audio_list.len());

    // Mark as playing
    {
        let mut playing = state.playing.lock().await;
        *playing = true;
    }

    // Play each audio in order
    let mut played = 0;
    let mut errors = Vec::new();

    for item in &audio_list {
        // Check for stop signal
        {
            let playing = state.playing.lock().await;
            if !*playing {
                println!("⏹️ Playback stopped");
                break;
            }
        }

        // Find audio file (try common extensions)
        let audio_path = find_audio_file(&state.audio_dir, &item.id);
        
        match audio_path {
            Some(path) => {
                println!("   ▶️ Playing: {}", item.id);
                
                // Decode audio to WAV
                let audio_data = match tokio::fs::read(&path).await {
                    Ok(data) => data,
                    Err(e) => {
                        errors.push(format!("{}: read error - {}", item.id, e));
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

                if let Some(wav_path) = state.processor.decode(&b64, format).await {
                    // Send to robot and wait for completion
                    state.player.send_audio(&wav_path).await;
                    
                    // Wait for playback signal (DONE from Python script)
                    // The player.send_audio already waits for DONE
                    
                    played += 1;

                    // Cleanup
                    let proc = state.processor.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        proc.cleanup(&wav_path);
                    });
                } else {
                    errors.push(format!("{}: decode failed", item.id));
                }
            }
            None => {
                errors.push(format!("{}: file not found", item.id));
                println!("   ⚠️ Not found: {}", item.id);
            }
        }
    }

    // Mark as not playing
    {
        let mut playing = state.playing.lock().await;
        *playing = false;
    }

    println!("✅ Checkpoint {} complete: {}/{} played", checkpoint, played, audio_list.len());

    if errors.is_empty() {
        Ok(Json(Response {
            status: "success".to_string(),
            message: format!("Played {} audios from {}", played, checkpoint),
        }))
    } else {
        Ok(Json(Response {
            status: "success".to_string(),
            message: format!(
                "Played {}/{} audios. Errors: {}",
                played,
                audio_list.len(),
                errors.join(", ")
            ),
        }))
    }
}

async fn stop_playback(State(state): State<AppState>) -> Json<Response> {
    let mut playing = state.playing.lock().await;
    *playing = false;
    Json(Response {
        status: "success".to_string(),
        message: "Playback stopped".to_string(),
    })
}

fn find_audio_file(audio_dir: &PathBuf, id: &str) -> Option<PathBuf> {
    let extensions = ["mp3", "wav", "ogg", "m4a", "flac"];
    
    for ext in &extensions {
        let path = audio_dir.join(format!("{}.{}", id, ext));
        if path.exists() {
            return Some(path);
        }
    }
    
    // Also try without extension (if id already has extension)
    let path = audio_dir.join(id);
    if path.exists() {
        return Some(path);
    }

    None
}

