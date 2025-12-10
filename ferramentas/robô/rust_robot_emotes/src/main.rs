mod ros;
mod web;

use futures_util::StreamExt;
use ros::ros_client::RobotClient;
use serde::{Deserialize, Serialize};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber;
use web::web_client::WebClient;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    ros_namespace: String,
}

// Comandos que podem ser enviados ao ROS via channel
#[derive(Debug, Clone, Copy)]
enum RosCommand {
    Hello,
    Stretch,
    Content,
    Wallow,
    Dance1,
    Dance2,
    Pose,
    Scrape,
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting robot emotes service");

    let config_content = std::fs::read_to_string("config/config.yaml")?;
    let config: Config = serde_yaml::from_str(&config_content)?;

    let ros_client = Arc::new(Mutex::new(RobotClient::new(
        &config.ros_namespace,
        "/api/sport/request",
    )?));

    let web_client = WebClient::new("0.0.0.0:3001");

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

    info!("System initialized. Waiting for Web signals...");

    let (tx, rx) = mpsc::channel::<RosCommand>();
    let rx = Arc::new(StdMutex::new(rx));

    let ros_for_commands = Arc::clone(&ros_client);
    let rx_clone = Arc::clone(&rx);
    
    std::thread::spawn(move || {
        // Cria um runtime tokio dentro desta thread para executar código async
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        loop {
            // Tenta receber comando
            let cmd_result = {
                let rx_guard = rx_clone.lock().unwrap();
                rx_guard.recv()
            };
            
            match cmd_result {
                Ok(RosCommand::Hello) => {
                    info!("*** COMMAND: Hello emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1016, "Hello").await {
                            Ok(()) => info!("Hello emote sent successfully"),
                            Err(e) => error!("Failed to send Hello: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Stretch) => {
                    info!("*** COMMAND: Stretch emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1017, "Stretch").await {
                            Ok(()) => info!("Stretch emote sent successfully"),
                            Err(e) => error!("Failed to send Stretch: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Content) => {
                    info!("*** COMMAND: Content emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1020, "Content").await {
                            Ok(()) => info!("Content emote sent successfully"),
                            Err(e) => error!("Failed to send Content: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Wallow) => {
                    info!("*** COMMAND: Wallow emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1021, "Wallow").await {
                            Ok(()) => info!("Wallow emote sent successfully"),
                            Err(e) => error!("Failed to send Wallow: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Dance1) => {
                    info!("*** COMMAND: Dance1 emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1022, "Dance1").await {
                            Ok(()) => info!("Dance1 emote sent successfully"),
                            Err(e) => error!("Failed to send Dance1: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Dance2) => {
                    info!("*** COMMAND: Dance2 emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1023, "Dance2").await {
                            Ok(()) => info!("Dance2 emote sent successfully"),
                            Err(e) => error!("Failed to send Dance2: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Pose) => {
                    info!("*** COMMAND: Pose emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1028, "Pose").await {
                            Ok(()) => info!("Pose emote sent successfully"),
                            Err(e) => error!("Failed to send Pose: {}", e),
                        }
                    });
                }
                Ok(RosCommand::Scrape) => {
                    info!("*** COMMAND: Scrape emote ***");
                    rt.block_on(async {
                        let guard = ros_for_commands.lock().await;
                        match guard.trigger_emote(1029, "Scrape").await {
                            Ok(()) => info!("Scrape emote sent successfully"),
                            Err(e) => error!("Failed to send Scrape: {}", e),
                        }
                    });
                }
                Err(_) => {
                    error!("Channel disconnected, exiting ROS command thread");
                    break;
                }
            }
        }
    });

    // Web emote callback: envia comandos de emote pelo channel
    let tx_emote = tx.clone();
    let web_emote_callback = move |emote: web::web_client::EmoteCommand| {
        use web::web_client::EmoteCommand;
        let tx = tx_emote.clone();
        
        match emote {
            EmoteCommand::Hello => {
                info!("*** WEB EMOTE: Hello ***");
                let _ = tx.send(RosCommand::Hello);
            }
            EmoteCommand::Stretch => {
                info!("*** WEB EMOTE: Stretch ***");
                let _ = tx.send(RosCommand::Stretch);
            }
            EmoteCommand::Content => {
                info!("*** WEB EMOTE: Content ***");
                let _ = tx.send(RosCommand::Content);
            }
            EmoteCommand::Wallow => {
                info!("*** WEB EMOTE: Wallow ***");
                let _ = tx.send(RosCommand::Wallow);
            }
            EmoteCommand::Dance1 => {
                info!("*** WEB EMOTE: Dance1 ***");
                let _ = tx.send(RosCommand::Dance1);
            }
            EmoteCommand::Dance2 => {
                info!("*** WEB EMOTE: Dance2 ***");
                let _ = tx.send(RosCommand::Dance2);
            }
            EmoteCommand::Pose => {
                info!("*** WEB EMOTE: Pose ***");
                let _ = tx.send(RosCommand::Pose);
            }
            EmoteCommand::Scrape => {
                info!("*** WEB EMOTE: Scrape ***");
                let _ = tx.send(RosCommand::Scrape);
            }
        }
    };

    web_client.start_emote_server(web_emote_callback).await;

    Ok(())
}