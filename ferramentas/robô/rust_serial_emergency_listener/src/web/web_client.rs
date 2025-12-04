use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

// Estado do botão de emergência web (igual ao serial)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

// Estado compartilhado entre handlers HTTP
#[derive(Clone)]
struct AppState {
    // Estado atual do botão virtual (pressionado/solto)
    button_state: Arc<Mutex<ButtonState>>,
    // Callback que será chamado quando houver mudança de estado
    // Recebe (estado_anterior, estado_novo)
    state_change_callback: Arc<dyn Fn(ButtonState, ButtonState) + Send + Sync>,
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

    /// Inicia servidor HTTP que monitora comandos de emergência via web
    /// 
    /// Endpoints:
    /// - POST /emergency/press   -> Simula botão pressionado (inicia DAMP contínuo)
    /// - POST /emergency/release -> Simula botão solto (para DAMP + envia RECOVER)
    /// - GET  /emergency/status  -> Retorna estado atual do botão
    pub async fn monitor_death_signal<F>(&self, callback: F)
    where
        F: Fn(ButtonState, ButtonState) + Send + Sync + 'static,
    {
        info!("Starting HTTP emergency server at {}", self.addr);

        let shared_state = AppState {
            button_state: Arc::new(Mutex::new(ButtonState::Released)),
            state_change_callback: Arc::new(callback),
        };

        // Define rotas da API REST
        let app = Router::new()
            .route("/emergency/press", post(handle_button_press))
            .route("/emergency/release", post(handle_button_release))
            .route("/emergency/status", axum::routing::get(get_button_status))
            .with_state(shared_state);

        let listener = tokio::net::TcpListener::bind(&self.addr).await.unwrap();
        
        info!("Web emergency endpoints ready:");
        info!("  POST   {}/emergency/press", self.addr);
        info!("  POST   {}/emergency/release", self.addr);
        info!("  GET    {}/emergency/status", self.addr);
        
        if let Err(e) = axum::serve(listener, app).await {
            warn!("Web server error: {}", e);
        }
    }
}

// Handler: POST /emergency/press
// Muda estado para "pressionado" e dispara callback (inicia DAMP contínuo)
async fn handle_button_press(State(state): State<AppState>) -> StatusCode {
    let mut current = state.button_state.lock().await;
    let prev_state = *current;
    
    // Só muda se estava Released
    if prev_state == ButtonState::Released {
        *current = ButtonState::Pressed;
        info!("WEB: Button PRESSED via HTTP");
        
        // Libera lock antes de chamar callback
        drop(current);
        
        // Notifica transição Released → Pressed
        (state.state_change_callback)(prev_state, ButtonState::Pressed);
        
        StatusCode::OK
    } else {
        warn!("WEB: Button already pressed, ignoring duplicate request");
        StatusCode::CONFLICT // 409 Conflict
    }
}

// Handler: POST /emergency/release
// Muda estado para "solto" e dispara callback (para DAMP + envia RECOVER)
async fn handle_button_release(State(state): State<AppState>) -> StatusCode {
    let mut current = state.button_state.lock().await;
    let prev_state = *current;
    
    // Só muda se estava Pressed
    if prev_state == ButtonState::Pressed {
        *current = ButtonState::Released;
        info!("WEB: Button RELEASED via HTTP");
        
        // Libera lock antes de chamar callback
        drop(current);
        
        // Notifica transição Pressed → Released
        (state.state_change_callback)(prev_state, ButtonState::Released);
        
        StatusCode::OK
    } else {
        warn!("WEB: Button already released, ignoring duplicate request");
        StatusCode::CONFLICT // 409 Conflict
    }
}

// Handler: GET /emergency/status
// Retorna estado atual do botão (para debug/monitoramento)
async fn get_button_status(State(state): State<AppState>) -> String {
    let current = state.button_state.lock().await;
    match *current {
        ButtonState::Pressed => "pressed".to_string(),
        ButtonState::Released => "released".to_string(),
    }
}