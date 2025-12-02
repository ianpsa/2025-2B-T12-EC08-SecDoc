use serialport::SerialPort;
use std::io::{BufRead, BufReader};
use std::time::Duration;
use tracing::{info, warn};


pub struct SerialHandler {
    reader: BufReader<Box<dyn SerialPort>>,
}

impl SerialHandler {
    /// Inicializa a conexão serial
    pub fn new(port_name: &str, baud_rate: u32) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Opening serial port {} with baud rate {}", port_name, baud_rate);
        let serial_port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(100))
            .open()?;
        let reader = BufReader::new(serial_port);
        
        info!("Serial port opened successfully");
        Ok(SerialHandler { reader })
    }
    
    // Read byte from TTY (blocking or async)
    pub fn read_button_state(&mut self) -> Result<Option<bool>, Box<dyn std::error::Error>> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        let trimmed = line.trim();
        
        match trimmed {
            "1" => Ok(Some(true)),   // Botão pressionado - emergência
            "0" => Ok(Some(false)),  // Botão liberado - normal
            "" => Ok(None),          // Unknown - ignorar
            _ => {
                warn!("Invalid input from serial port: '{}'", trimmed);
                Ok(None)
            }
        }
    }
    
    pub async fn monitor_emergency_signal<F>(&mut self, mut callback: F) 
    where 
        F: FnMut(bool) + Send,
    {
        let mut last_state: Option<bool> = None;
                
        loop {
            match self.read_button_state() {
                Ok(Some(state)) => {
                    if last_state != Some(state) {
                        info!("Change detected: {} -> {}", 
                              last_state.map_or("None".to_string(), |s| s.to_string()), 
                              state);
                        
                        if state {
                            info!("Emergency button pressed!");
                            callback(state);
                        } else {
                            info!("Emergency button released");
                        }
                        
                        last_state = Some(state);
                    }
                },
                Ok(None) => {
                },
                Err(e) => {
                    warn!("Error reading serial port: {}. Trying again...", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            
            // Pequeno delay para não sobrecarregar CPU
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}