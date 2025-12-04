mod ros;
mod serial;
mod web;

use serial::config::load_config;
use futures_util::StreamExt;
use ros::ros_client::EmergencyStopClient;
use serial::serial::SerialHandler;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber;
use web::web_client::WebClient;


// Helper function to handle the actual ROS call safely
// This prevents code duplication in the callbacks
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

    let ros_for_serial = Arc::clone(&ros_client);
    let serial_callback = move || {
        let c = Arc::clone(&ros_for_serial);
        tokio::spawn(async move {
            execute_emergency_stop(c, "SERIAL").await;
        });
    };

    let ros_for_web = Arc::clone(&ros_client);
    let web_callback = move || {
        let c = Arc::clone(&ros_for_web);
        tokio::spawn(async move {
            execute_emergency_stop(c, "WEB").await;
        });
    };

    tokio::join!(
        serial.monitor_emergency_signal(serial_callback),
        web_client.monitor_death_signal(web_callback)
    );

    Ok(())
}