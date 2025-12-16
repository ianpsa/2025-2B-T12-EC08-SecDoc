mod ros;
mod serial;
mod web;

use serial::config::load_config;
use futures_util::StreamExt;
use ros::ros_client::EmergencyStopClient;
use serial::serial::{SerialHandler, State};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber;
use web::web_client::WebClient;

// Comandos que podem ser enviados ao ROS via channel
#[derive(Debug, Clone, Copy)]
enum RosCommand {
    StartDamp,      // Inicia envio contínuo de DAMP
    StopAndRecover, // Para DAMP e envia RECOVER
}

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
                client.node.spin_once(std::time::Duration::from_millis(1));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
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

    // Canal para enviar comandos ROS de forma thread-safe (blocking channel)
    let (tx, rx) = mpsc::channel::<RosCommand>();
    let rx = Arc::new(StdMutex::new(rx));
    
    // Thread separada que processa comandos ROS (std::thread, não tokio::spawn!)
    // Isso resolve o problema Send/Sync porque não cruza boundary do tokio
    let ros_for_commands = Arc::clone(&ros_client);
    let rx_clone = Arc::clone(&rx);
    
    std::thread::spawn(move || {
        // Cria um runtime tokio dentro desta thread para executar código async
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut damp_active = false;
        
        loop {
            // Tenta receber comando (timeout de 100ms)
            let cmd_result = {
                let rx_guard = rx_clone.lock().unwrap();
                rx_guard.recv_timeout(std::time::Duration::from_millis(10))
            };
            
            match cmd_result {
                Ok(RosCommand::StartDamp) => {
                    info!("*** COMMAND: Start continuous DAMP ***");
                    damp_active = true;
                }
                Ok(RosCommand::StopAndRecover) => {
                    info!("*** COMMAND: Stop DAMP and send RECOVER ***");
                    damp_active = false;
                    
                    // Usa o runtime tokio para executar código async
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_recovery().await {
                            Ok(()) => info!("Recovery command sent successfully"),
                            Err(e) => error!("Failed to send RECOVER: {}", e),
                        }
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Timeout normal - envia DAMP se ativo
                    if damp_active {
                        info!("Sending continuous DAMP command");
                        rt.block_on(async {
                            let guard = ros_for_commands.lock().await;
                            match guard.trigger_emergency_stop(true).await {
                                Ok(()) => {},
                                Err(e) => error!("Failed to send DAMP: {}", e),
                            }
                        });
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    error!("Channel disconnected, exiting ROS command thread");
                    break;
                }
            }
        }
    });

    // Serial callback: envia comandos pelo channel
    let tx_serial = tx.clone();
    let serial_callback = move |prev_state: State, new_state: State| {
        let tx = tx_serial.clone();
        
        // Spawn não é necessário - send é blocking e rápido
        match (prev_state, new_state) {
            (State::OFF, State::ON) => {
                info!("*** SERIAL BUTTON PRESSED ***");
                let _ = tx.send(RosCommand::StartDamp);
            }
            (State::ON, State::OFF) => {
                info!("*** SERIAL BUTTON RELEASED ***");
                let _ = tx.send(RosCommand::StopAndRecover);
            }
            _ => {}
        }
    };

    // Web callback: envia comandos pelo channel
    let tx_web = tx.clone();
    let web_callback = move |prev_state: web::web_client::ButtonState, new_state: web::web_client::ButtonState| {
        use web::web_client::ButtonState;
        let tx = tx_web.clone();
        
        // Spawn não é necessário - send é blocking e rápido
        match (prev_state, new_state) {
            (ButtonState::Released, ButtonState::Pressed) => {
                info!("*** WEB BUTTON PRESSED ***");
                let _ = tx.send(RosCommand::StartDamp);
            }
            (ButtonState::Pressed, ButtonState::Released) => {
                info!("*** WEB BUTTON RELEASED ***");
                let _ = tx.send(RosCommand::StopAndRecover);
            }
            _ => {}
        }
    };

    tokio::join!(
        serial.monitor_emergency_signal(serial_callback),
        web_client.monitor_death_signal(web_callback)
    );

    Ok(())
}