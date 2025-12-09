use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

// Emote commands that can be triggered via web
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmoteCommand {
    Hello,
    Stretch,
    Content,
    Wallow,
    Dance1,
    Dance2,
    Pose,
    Scrape,
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

    /// Inicia servidor HTTP que monitora comandos de emote via web
    /// Endpoints:
    /// - POST /emote/hello -> faz movimento de oi
    /// - POST /emote/stretch -> se espreguiça
    /// - POST /emote/content -> expressão feliz
    /// - POST /emote/wallow -> rola no chão
    /// - POST /emote/dance1 -> dança 1
    /// - POST /emote/dance2 -> dança 2
    /// - POST /emote/pose -> pose
    /// - POST /emote/scrape -> esfrega a bunda no chão
    /// - GET /emote/status -> exibe status deste daemon
    pub async fn start_emote_server<F>(&self, callback: F)
    where
        F: Fn(EmoteCommand) + Send + Sync + 'static,
    {
        info!("Starting HTTP emote server at {}", self.addr);

        let shared_state = AppState {
            emote_callback: Arc::new(callback),
        };

        let app = Router::new()
            .route("/emote/hello", post(handle_hello))
            .route("/emote/stretch", post(handle_stretch))
            .route("/emote/content", post(handle_content))
            .route("/emote/wallow", post(handle_wallow))
            .route("/emote/dance1", post(handle_dance1))
            .route("/emote/dance2", post(handle_dance2))
            .route("/emote/pose", post(handle_pose))
            .route("/emote/scrape", post(handle_scrape))
            .route("/emote/status", axum::routing::get(get_status))
            .with_state(shared_state);

        let listener = tokio::net::TcpListener::bind(&self.addr).await.unwrap();
        
        info!("Web emote endpoints ready:");
        info!("  POST   {}/emote/hello", self.addr);
        info!("  POST   {}/emote/stretch", self.addr);
        info!("  POST   {}/emote/content", self.addr);
        info!("  POST   {}/emote/wallow", self.addr);
        info!("  POST   {}/emote/dance1", self.addr);
        info!("  POST   {}/emote/dance2", self.addr);
        info!("  POST   {}/emote/pose", self.addr);
        info!("  POST   {}/emote/scrape", self.addr);
        info!("  GET    {}/emote/status", self.addr);
        
        if let Err(e) = axum::serve(listener, app).await {
            warn!("Web server error: {}", e);
        }
    }
}

struct AppState<F>
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    emote_callback: Arc<F>,
}

impl<F> Clone for AppState<F>
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            emote_callback: Arc::clone(&self.emote_callback),
        }
    }
}

// Emote handlers
async fn handle_hello<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Hello emote triggered");
    (state.emote_callback)(EmoteCommand::Hello);
    StatusCode::OK
}

async fn handle_stretch<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Stretch emote triggered");
    (state.emote_callback)(EmoteCommand::Stretch);
    StatusCode::OK
}

async fn handle_content<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Content emote triggered");
    (state.emote_callback)(EmoteCommand::Content);
    StatusCode::OK
}

async fn handle_wallow<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Wallow emote triggered");
    (state.emote_callback)(EmoteCommand::Wallow);
    StatusCode::OK
}

async fn handle_dance1<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Dance1 emote triggered");
    (state.emote_callback)(EmoteCommand::Dance1);
    StatusCode::OK
}

async fn handle_dance2<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Dance2 emote triggered");
    (state.emote_callback)(EmoteCommand::Dance2);
    StatusCode::OK
}

async fn handle_pose<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Pose emote triggered");
    (state.emote_callback)(EmoteCommand::Pose);
    StatusCode::OK
}

async fn handle_scrape<F>(State(state): State<AppState<F>>) -> StatusCode
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    info!("WEB: Scrape emote triggered");
    (state.emote_callback)(EmoteCommand::Scrape);
    StatusCode::OK
}

// Handler: GET /emote/status
async fn get_status<F>() -> String
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    "Robot emote service is running".to_string()
}