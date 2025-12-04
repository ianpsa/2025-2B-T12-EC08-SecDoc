use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Router,
};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    callback: Arc<dyn Fn() + Send + Sync>,
}

pub struct WebClient {
    addr: String,
}

impl WebClient {
    pub fn new(addr: &str) -> Self {
        info!("Configured web client on: {}", addr);
        Self {
            addr: addr.to_string(),
        }
    }

    pub async fn monitor_death_signal<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        info!("Starting HTTP server at {}", self.addr);

        let shared_state = AppState {
            callback: Arc::new(callback),
        };

        let app = Router::new()
            .route("/kill", post(trigger_death))
            .with_state(shared_state);

        let listener = tokio::net::TcpListener::bind(&self.addr).await.unwrap();
        
        if let Err(e) = axum::serve(listener, app).await {
            warn!("Web server error: {}", e);
        }
    }
}

async fn trigger_death(State(state): State<AppState>) -> StatusCode {
    warn!("Received HTTP Death Signal!");
    
    (state.callback)();

    StatusCode::OK
}