mod serial;
mod config;
mod ros_client;

use futures_util::StreamExt;
use serial::SerialHandler;
use config::load_config;
use ros_client::EmergencyStopClient;
use tracing::{info, error, warn};
use tracing_subscriber;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

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
    
    // Adicionar flag para controlar se há uma chamada em andamento
    let is_processing = Arc::new(AtomicBool::new(false));
    
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
    let is_processing_clone = is_processing.clone();
    serial_handler.monitor_emergency_signal(move |state| {
        if state {
            // Verificar se já há uma chamada em andamento
            if is_processing_clone.compare_exchange(
                false, 
                true, 
                Ordering::SeqCst, 
                Ordering::SeqCst
            ).is_ok() {
                let ros_client = ros_client.clone();
                let is_processing_inner = is_processing_clone.clone();
                
                tokio::spawn(async move {
                    info!("Processing emergency button press...");
                    
                    // Timeout de 2 segundos para a chamada (reduzido para resposta rápida)
                    match tokio::time::timeout(
                        tokio::time::Duration::from_millis(150),
                        ros_client.trigger_emergency_stop(true)
                    ).await {
                        Ok(Ok(())) => {
                            info!("Emergency stop executed successfully");
                        }
                        Ok(Err(e)) => {
                            error!("Error triggering emergency stop: {}", e);
                        }
                        Err(_) => {
                            error!("Emergency stop call timed out after 150 milliseconds");
                        }
                    }
                    
                    // Liberar flag
                    is_processing_inner.store(false, Ordering::SeqCst);
                });
            } else {
                warn!("Emergency button pressed but previous call still processing - ignoring");
            }
        }
    }).await;
    
    Ok(())
}