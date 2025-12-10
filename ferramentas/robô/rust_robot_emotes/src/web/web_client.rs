use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Router,
};
use axum_client_ip::{InsecureClientIp, SecureClientIpSource};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
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

pub struct RateLimiter {
    last_request: Mutex<Option<Instant>>,
    timeout: Duration,
}

impl RateLimiter {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            last_request: Mutex::new(None),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    pub async fn check_and_update(&self, ip: IpAddr) -> Result<(), Duration> {
        let mut last_request = self.last_request.lock().await;
        let now = Instant::now();

        if let Some(last_request_time) = *last_request {
            let elapsed = now.duration_since(last_request_time);
            if elapsed < self.timeout {
                let remaining = self.timeout - elapsed;
                return Err(remaining);
            }
        }

        *last_request = Some(now);
        Ok(())
    }
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
            rate_limiter: Arc::new(RateLimiter::new(20)),
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
            .layer(SecureClientIpSource::ConnectInfo.into_extension())
            .with_state(shared_state);

        info!("Web emote endpoints ready:");
        info!("  POST   {}/emote/hello", self.addr);
        info!("  POST   {}/emote/stretch", self.addr);
        info!("  POST   {}/emote/content", self.addr);
        info!("  POST   {}/emote/wallow", self.addr);
        info!("  POST   {}/emote/dance1", self.addr);
        info!("  POST   {}/emote/dance2", self.addr);
        info!("  POST   {}/emote/pose", self.addr);
        info!("  POST   {}/emote/scrape", self.addr);
        
        let listener = tokio::net::TcpListener::bind(&self.addr).await.unwrap();
        if let Err(e) = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>()
        ).await {
            warn!("Web server error: {}", e);
        }
    }
}

struct AppState<F>
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    emote_callback: Arc<F>,
    rate_limiter: Arc<RateLimiter>,
}

impl<F> Clone for AppState<F>
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            emote_callback: Arc::clone(&self.emote_callback),
            rate_limiter: Arc::clone(&self.rate_limiter),
        }
    }
}

// Emote handlers
async fn handle_hello<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Hello emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Hello);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_stretch<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Stretch emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Stretch);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_content<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Content emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Content);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_wallow<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Wallow emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Wallow);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_dance1<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Dance1 emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Dance1);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_dance2<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Dance2 emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Dance2);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_pose<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Pose emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Pose);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}

async fn handle_scrape<F>(
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState<F>>,
) -> (StatusCode, String)
where
    F: Fn(EmoteCommand) + Send + Sync + 'static,
{
    match state.rate_limiter.check_and_update(ip).await {
        Ok(()) => {
            info!("WEB: Scrape emote triggered from IP: {}", ip);
            (state.emote_callback)(EmoteCommand::Scrape);
            (StatusCode::OK, "Emote triggered".to_string())
        }
        Err(remaining) => {
            warn!("WEB: Rate limit exceeded for IP: {} ({}s remaining)", ip, remaining.as_secs());
            (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Please wait {} seconds", remaining.as_secs())
            )
        }
    }
}