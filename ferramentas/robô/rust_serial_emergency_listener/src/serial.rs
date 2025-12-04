use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_serial::SerialPortBuilderExt; // Trait required to open ports
use tracing::{error, info, warn};

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub enum State {
    ON,
    #[default]
    OFF,
}

// changed From to TryFrom to avoid panicking on noise
impl TryFrom<String> for State {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.trim() {
            "0" => Ok(Self::OFF),
            "1" => Ok(Self::ON),
            // Handle noise/garbage without crashing the thread
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
        F: Fn() + Send + 'static,
    {
        info!("Opening serial port {} at {}", self.port_name, self.baud_rate);

        // 1. Open the port using tokio-serial, not File::open
        let port = match tokio_serial::new(&self.port_name, self.baud_rate).open_native_async() {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to open serial port: {}", e);
                return;
            }
        };

        // 2. Wrap in BufReader for line-by-line reading
        let reader = BufReader::new(port);
        let mut lines = reader.lines();
        let mut current_state = State::default();

        // 3. Loop over lines asynchronously
        while let Ok(Some(line_str)) = lines.next_line().await {
            // 4. Safely parse the state
            match State::try_from(line_str) {
                Ok(new_state) => {
                    // Only trigger if state actually changed
                    if new_state != current_state {
                        info!("State change detected: {:?} -> {:?}", current_state, new_state);
                        current_state = new_state;

                        if current_state == State::ON {
                            callback();
                        }
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

// Example Usage Scaffolding
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    // On Linux usually "/dev/ttyUSB0" or "/dev/ttyACM0"
    // On Windows usually "COM3"
    let handler = SerialHandler::new("/dev/ttyUSB0", 9600);
    
    handler.monitor_emergency_signal(|| {
        println!("*** EMERGENCY SIGNAL RECEIVED! ***");
    }).await;
}