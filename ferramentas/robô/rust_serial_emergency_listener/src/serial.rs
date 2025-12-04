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
            _ => panic!("Unexpected read from serial buffer")
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
        let path = self.path.clone();
        
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
    }
}
