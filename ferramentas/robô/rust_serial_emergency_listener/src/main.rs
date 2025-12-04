mod ros;
mod serial;
mod web;

use serial::config::load_config;
use futures_util::StreamExt;
use ros::ros_client::EmergencyStopClient;
use serial::serial::{SerialHandler, State};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber;
use web::web_client::WebClient;

async fn execute_emergency_stop(client: Arc<Mutex<EmergencyStopClient>>, source: &str) {
    info!("*** {} KILL SWITCH TRIGGERED ***", source);
    let guard = client.lock().await;
    
    match guard.trigger_emergency_stop(true).await {
        Ok(()) => info!("ROS Emergency Stop executed successfully via {}", source),
        Err(e) => error!("Failed to trigger ROS Stop via {}: {}", source, e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting emergency stop service");

    let config = load_config("config/config.yaml")?;

    let ros_client = Arc::new(Mutex::new(EmergencyStopClient::new(
        &config.ros_namespace,
        "/api/sport/request",
    )?));

    let serial = SerialHandler::new(&config.serial_port, 9600); 
    let web_client = WebClient::new("0.0.0.0:3000");

    let ros_client_spin = Arc::clone(&ros_client);
    tokio::spawn(async move {
        loop {
            {
                let mut client = ros_client_spin.lock().await;
                client.node.spin_once(std::time::Duration::from_millis(10));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;
    let _signals_handle = signals.handle();
    tokio::spawn(async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM | SIGINT => {
                    info!("Shutdown signal received, exiting...");
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    info!("System initialized. Waiting for Serial or Web signals...");

    // Estado compartilhado: indica se o botão está pressionado
    // e a task que envia damp continuamente enquanto pressionado
    let button_pressed = Arc::new(Mutex::new(false));
    let button_pressed_for_loop = Arc::clone(&button_pressed);
    let ros_for_damp_loop = Arc::clone(&ros_client);
    
    // Task que fica enviando damp continuamente enquanto o botão estiver pressionado
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            let is_pressed = *button_pressed_for_loop.lock().await;
            
            if is_pressed {
                info!("Sending continuous DAMP command (button pressed)");
                let guard = ros_for_damp_loop.lock().await;
                match guard.trigger_emergency_stop(true).await {
                    Ok(()) => {},
                    Err(e) => error!("Failed to send DAMP: {}", e),
                }
            }
        }
    });

    let ros_for_serial = Arc::clone(&ros_client);
    let button_state = Arc::clone(&button_pressed);
    
    let serial_callback = move |prev_state: State, new_state: State| {
        let c = Arc::clone(&ros_for_serial);
        let btn = Arc::clone(&button_state);
        
        tokio::spawn(async move {
            match (prev_state, new_state) {
                // Botão foi pressionado: ativa flag para enviar damp continuamente
                (State::OFF, State::ON) => {
                    info!("*** SERIAL BUTTON PRESSED - Starting continuous DAMP ***");
                    *btn.lock().await = true;
                }
                // Botão foi solto: desativa flag e envia RECOVER uma única vez
                (State::ON, State::OFF) => {
                    info!("*** SERIAL BUTTON RELEASED - Sending RECOVER once ***");
                    *btn.lock().await = false;
                    
                    let guard = c.lock().await;
                    match guard.trigger_recovery().await {
                        Ok(()) => info!("Recovery command sent successfully"),
                        Err(e) => error!("Failed to send RECOVER: {}", e),
                    }
                }
                _ => {}
            }
        });
    };

    // Web callback: mesma lógica do serial (transições de estado)
    let ros_for_web = Arc::clone(&ros_client);
    let button_state_web = Arc::clone(&button_pressed);
    
    let web_callback = move |prev_state: web::web_client::ButtonState, new_state: web::web_client::ButtonState| {
        use web::web_client::ButtonState;
        
        let c = Arc::clone(&ros_for_web);
        let btn = Arc::clone(&button_state_web);
        
        tokio::spawn(async move {
            match (prev_state, new_state) {
                // Botão web pressionado: ativa flag para enviar damp continuamente
                (ButtonState::Released, ButtonState::Pressed) => {
                    info!("*** WEB BUTTON PRESSED - Starting continuous DAMP ***");
                    *btn.lock().await = true;
                }
                // Botão web solto: desativa flag e envia RECOVER uma única vez
                (ButtonState::Pressed, ButtonState::Released) => {
                    info!("*** WEB BUTTON RELEASED - Sending RECOVER once ***");
                    *btn.lock().await = false;
                    
                    let guard = c.lock().await;
                    match guard.trigger_recovery().await {
                        Ok(()) => info!("Recovery command sent successfully (from WEB)"),
                        Err(e) => error!("Failed to send RECOVER from WEB: {}", e),
                    }
                }
                _ => {}
            }
        });
    };

    tokio::join!(
        serial.monitor_emergency_signal(serial_callback),
        web_client.monitor_death_signal(web_callback)
    );

    Ok(())
}