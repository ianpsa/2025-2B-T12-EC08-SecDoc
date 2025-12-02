mod serial;
mod config;
mod ros_client;

use futures_util::StreamExt;
use serial::SerialHandler;
use config::load_config;
use ros_client::EmergencyStopClient;
use tracing::{info, error};
use tracing_subscriber;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("Starting emergency stop service");
    
    // 1. Carregar configuração
    let config = load_config("config/config.yaml")?;
    
    // 2. Inicializar cliente ROS2
    let ros_client = EmergencyStopClient::new(
        &config.ros_namespace,
        &config.ros_service_name,
    )?;
    
    // 3. Inicializar handler serial
    let mut serial_handler = SerialHandler::new(
        &config.serial_port,
        config.baud_rate,
    )?;
    
    // 4. Spawn task para manter ROS2 spinning
    let ros_client_spin = ros_client.clone();
    tokio::spawn(async move {
        ros_client_spin.spin().await;
    });
    
    // 5. Configurar tratamento de sinais para shutdown gracioso
    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;
    let signals_handle = signals.handle();
    
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
    
    info!("System initialized successfully");
    info!("Waiting for emergency button events...");
    
    // 6. Loop principal: monitorar serial e acionar serviço
    serial_handler.monitor_emergency_signal(move |state| {
        if state {
            let ros_client = ros_client.clone();
            tokio::spawn(async move {
                if let Err(e) = ros_client.trigger_emergency_stop(true).await {
                    error!("Error triggering emergency stop: {}", e);
                }
            });
        }
    }).await;
    
    Ok(())
}