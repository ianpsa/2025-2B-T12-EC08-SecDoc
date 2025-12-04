use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Router,
};
use std::sync::Arc;
use tracing::{info, warn};

// We wrap the callback in a struct so we can pass it safely 
// into the HTTP handler threads.
#[derive(Clone)]
struct AppState {
    // Arc<dyn ...> allows us to share the closure across threads
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

        // 1. Create the shared state with the callback
        let shared_state = AppState {
            callback: Arc::new(callback),
        };

        // 2. Define the route
        // POST request to http://<addr>/kill will trigger the callback
        let app = Router::new()
            .route("/kill", post(trigger_death))
            .with_state(shared_state);

        // 3. Bind and serve
        let listener = tokio::net::TcpListener::bind(&self.addr).await.unwrap();
        
        if let Err(e) = axum::serve(listener, app).await {
            warn!("Web server error: {}", e);
        }
    }
}

// The actual handler function that runs when you hit the endpoint
async fn trigger_death(State(state): State<AppState>) -> StatusCode {
    warn!("Received HTTP Death Signal!");
    
    // Execute the callback
    (state.callback)();

    // Return 200 OK
    StatusCode::OK
}