use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_serial::SerialPortBuilderExt;
use tracing::{error, info, warn};

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub enum State {
    ON,
    #[default]
    OFF,
}

impl TryFrom<String> for State {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.trim() {
            "0" => Ok(Self::OFF),
            "1" => Ok(Self::ON),
            other => Err(format!("Unknown signal received: '{}'", other)),
        }
    }
}

pub struct SerialHandler {
    port_name: String,
    baud_rate: u32,
}

impl SerialHandler {
    pub fn new(port_name: &str, baud_rate: u32) -> Self {
        Self {
            port_name: port_name.to_string(),
            baud_rate,
        }
    }

    pub async fn monitor_emergency_signal<F>(&self, callback: F)
    where
        F: Fn(State, State) + Send + 'static, // Agora recebe (estado_anterior, estado_novo)
    {
        info!("Opening serial port {} at {}", self.port_name, self.baud_rate);

        let port = match tokio_serial::new(&self.port_name, self.baud_rate).open_native_async() {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to open serial port: {}", e);
                return;
            }
        };

        let reader = BufReader::new(port);
        let mut lines = reader.lines();
        let mut current_state = State::default();

        while let Ok(Some(line_str)) = lines.next_line().await {
            match State::try_from(line_str) {
                Ok(new_state) => {
                    if new_state != current_state {
                        info!("State change detected: {:?} -> {:?}", current_state, new_state);
                        
                        // Chama callback com estado anterior e novo estado
                        callback(current_state, new_state);
                        
                        current_state = new_state;
                    }
                }
                Err(e) => {
                    warn!("Serial Parse Warning: {}", e);
                }
            }
        }
        
        warn!("Serial connection closed or stream ended.");
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let handler = SerialHandler::new("/dev/ttyUSB0", 9600);
    
    handler.monitor_emergency_signal(|prev, new| {
        println!("*** STATE CHANGE: {:?} -> {:?} ***", prev, new);
    }).await;
}