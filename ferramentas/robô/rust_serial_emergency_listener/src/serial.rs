use std::io::BufRead;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{read, File};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Default, Eq, PartialEq)]
pub enum State {
    ON,
    #[default]
    OFF,
}

impl From<String> for State {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str().trim() {
            "0" => Self::OFF,
            "1" => Self::ON,
            _ => panic!("unexpected read from serial buffer"),
        }
    }
}

#[derive(Default)]
pub struct SerialHandler {
    path: PathBuf,
}

impl SerialHandler {
    pub fn new(port_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Opening serial port {}", port_name);

        Ok(SerialHandler {
            path: PathBuf::from_str(port_name)?,
        })
    }

    pub async fn monitor_emergency_signal<F>(&self, callback: F)
    where
        F: Fn() + Send + 'static,
    {
        // let mut last_state: Option<bool> = None;
        // let mut last_trigger_time: Option<std::time::Instant> = None;
        // let mut last_valid_read_time: std::time::Instant = std::time::Instant::now();
        // let debounce_duration = Duration::from_millis(100); // Reduzido para 100ms para resposta rápida
        // let state_reset_timeout = Duration::from_secs(3); // Reduzido para 150ms

        // loop {
        //     match self.read_button_state() {
        //         Ok(Some(state)) => {
        //             // Atualizar tempo da última leitura válida
        //             last_valid_read_time = std::time::Instant::now();

        //             if last_state != Some(state) {
        //                 info!(
        //                     "Change detected: {} -> {}",
        //                     last_state.map_or("None".to_string(), |s| s.to_string()),
        //                     state
        //                 );

        //                 if state {
        //                     // Verificar debounce: só dispara se passou tempo suficiente
        //                     let should_trigger = match last_trigger_time {
        //                         Some(last_time) => last_time.elapsed() >= debounce_duration,
        //                         None => true,
        //                     };

        //                     if should_trigger {
        //                         info!("Emergency button pressed! Triggering callback...");
        //                         callback(state);
        //                         last_trigger_time = Some(std::time::Instant::now());
        //                     } else {
        //                         info!("Emergency button press ignored (debounce)");
        //                     }
        //                 } else {
        //                     info!("Emergency button released");
        //                 }

        //                 last_state = Some(state);
        //             }
        //         }
        //         Ok(None) => {
        //             // Linha vazia ou inválida - verificar timeout
        //             if last_state.is_some() && last_valid_read_time.elapsed() >= state_reset_timeout
        //             {
        //                 info!(
        //                     "No valid data received for {:?}, resetting state to None",
        //                     state_reset_timeout
        //                 );
        //                 last_state = None;
        //                 last_valid_read_time = std::time::Instant::now();
        //             }
        //         }
        //         Err(e) => {
        //             warn!("Error reading serial port: {}. Trying again...", e);

        //             // Em caso de erro também verificar timeout
        //             if last_state.is_some() && last_valid_read_time.elapsed() >= state_reset_timeout
        //             {
        //                 info!("Timeout after error, resetting state to None");
        //                 last_state = None;
        //                 last_valid_read_time = std::time::Instant::now();
        //             }

        //             tokio::time::sleep(Duration::from_secs(1)).await;
        //             continue; // Skip the normal sleep at the bottom
        //         }
        //     }

        //     tokio::time::sleep(Duration::from_millis(150)).await;
        // }

        let path = self.path.clone();
        tokio::spawn(async move {
            let mut state: State = Default::default();
            if let Ok(file) = File::open(path).await {
                let mut lines = BufReader::new(file).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let new: State = line.into();

                    if new.ne(&state) {
                        state = new;
                        if state == State::ON {
                            callback()
                        }
                    }
                }
            }
        });
    }
}
