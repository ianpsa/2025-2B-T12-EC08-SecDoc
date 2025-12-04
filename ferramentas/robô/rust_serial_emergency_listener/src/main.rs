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

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("Starting emergency stop service");
    
    // 1. Carregar configuração
    let config = load_config("config/config.yaml")?;
    
    // 2. Inicializar cliente ROS2 wrapped em Arc<Mutex<>>
    let ros_client = Arc::new(Mutex::new(
        EmergencyStopClient::new(
            &config.ros_namespace,
            "/api/sport/request",  // Topic name
        )?
    ));
    
    // 3. Inicializar handler serial
    let mut serial_handler = SerialHandler::new(
        &config.serial_port,
        config.baud_rate,
    )?;
    
    // 4. Spawn task para manter ROS2 spinning
    let ros_client_spin = Arc::clone(&ros_client);
    tokio::spawn(async move {
        loop {
            {
                let mut client = ros_client_spin.lock().await;
                client.node.spin_once(std::time::Duration::from_millis(100));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });
    
    // 5. Configurar tratamento de sinais para shutdown gracioso
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
    
    info!("System initialized successfully");
    info!("Waiting for emergency button events...");
    
    // 6. Loop principal: monitorar serial e acionar publisher
    let ros_client_monitor = Arc::clone(&ros_client);
    serial_handler.monitor_emergency_signal(move |state| {
        if state {
            let ros_client = Arc::clone(&ros_client_monitor);
            
            // Use tokio::task::spawn_blocking instead of tokio::spawn
            // This moves the work to a blocking thread pool
            tokio::task::spawn_blocking(move || {
                info!("Processing emergency button press...");
                
                // Use block_on to run async code in blocking context
                let runtime = tokio::runtime::Handle::current();
                runtime.block_on(async {
                    let mut client = ros_client.lock().await;
                    match client.trigger_emergency_stop(true).await {
                        Ok(()) => {
                            info!("Emergency stop executed successfully");
                        }
                        Err(e) => {
                            error!("Error triggering emergency stop: {}", e);
                        }
                    }
                });
            });
        }
    }).await;
    
    Ok(())
}