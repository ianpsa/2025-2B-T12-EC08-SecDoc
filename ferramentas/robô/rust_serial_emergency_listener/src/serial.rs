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
            .timeout(Duration::from_millis(10))  // Timeout reduzido para 10ms para resposta rápida
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
        let mut last_trigger_time: Option<std::time::Instant> = None;
        let mut last_valid_read_time: std::time::Instant = std::time::Instant::now();
        let debounce_duration = Duration::from_millis(100); // Reduzido para 100ms para resposta rápida
        let state_reset_timeout = Duration::from_secs(3); // Reduzido para 150ms
                
        loop {
            match self.read_button_state() {
                Ok(Some(state)) => {
                    // Atualizar tempo da última leitura válida
                    last_valid_read_time = std::time::Instant::now();
                    
                    if last_state != Some(state) {
                        info!("Change detected: {} -> {}", 
                              last_state.map_or("None".to_string(), |s| s.to_string()), 
                              state);
                        
                        if state {
                            // Verificar debounce: só dispara se passou tempo suficiente
                            let should_trigger = match last_trigger_time {
                                Some(last_time) => last_time.elapsed() >= debounce_duration,
                                None => true,
                            };
                            
                            if should_trigger {
                                info!("Emergency button pressed! Triggering callback...");
                                callback(state);
                                last_trigger_time = Some(std::time::Instant::now());
                            } else {
                                info!("Emergency button press ignored (debounce)");
                            }
                        } else {
                            info!("Emergency button released");
                        }
                        
                        last_state = Some(state);
                    }
                },
                Ok(None) => {
                    // Linha vazia ou inválida - verificar timeout
                    if last_state.is_some() && last_valid_read_time.elapsed() >= state_reset_timeout {
                        info!("No valid data received for {:?}, resetting state to None", state_reset_timeout);
                        last_state = None;
                        last_valid_read_time = std::time::Instant::now();
                    }
                },
                Err(e) => {
                    warn!("Error reading serial port: {}. Trying again...", e);
                    
                    // Em caso de erro também verificar timeout
                    if last_state.is_some() && last_valid_read_time.elapsed() >= state_reset_timeout {
                        info!("Timeout after error, resetting state to None");
                        last_state = None;
                        last_valid_read_time = std::time::Instant::now();
                    }
                    
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
            
            // Delay mínimo para não sobrecarregar CPU
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}