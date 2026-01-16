//! DMX-512 bridge (USB DMX interfaces)
//!
//! Supports common USB-DMX interfaces like ENTTEC DMX USB Pro

use async_trait::async_trait;
use parking_lot::Mutex;
use clasp_core::{Message, SetMessage, Value};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};

/// DMX interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmxInterfaceType {
    /// ENTTEC DMX USB Pro
    EnttecPro,
    /// ENTTEC Open DMX USB
    EnttecOpen,
    /// Generic FTDI-based
    Ftdi,
    /// Virtual (for testing)
    Virtual,
}

impl Default for DmxInterfaceType {
    fn default() -> Self {
        Self::Virtual
    }
}

/// DMX bridge configuration
#[derive(Debug, Clone)]
pub struct DmxBridgeConfig {
    /// Serial port path
    pub port: Option<String>,
    /// Interface type
    pub interface_type: DmxInterfaceType,
    /// Universe number (for multi-universe setups)
    pub universe: u16,
    /// Address namespace
    pub namespace: String,
    /// Refresh rate in Hz
    pub refresh_rate: f64,
}

impl Default for DmxBridgeConfig {
    fn default() -> Self {
        Self {
            port: None,
            interface_type: DmxInterfaceType::Virtual,
            universe: 0,
            namespace: "/dmx".to_string(),
            refresh_rate: 44.0, // Standard DMX refresh
        }
    }
}

/// DMX sender for thread-safe output
struct DmxSender {
    tx: std::sync::mpsc::Sender<DmxCommand>,
}

enum DmxCommand {
    SetChannel(u16, u8),
    SetFrame([u8; 512]),
    Stop,
}

/// DMX-512 to SignalFlow bridge
pub struct DmxBridge {
    config: BridgeConfig,
    dmx_config: DmxBridgeConfig,
    running: Arc<Mutex<bool>>,
    tx: Option<mpsc::Sender<BridgeEvent>>,
    dmx_sender: Option<DmxSender>,
    /// Current DMX values
    dmx_state: Arc<Mutex<[u8; 512]>>,
    /// Output thread handle
    _output_thread: Option<std::thread::JoinHandle<()>>,
}

impl DmxBridge {
    pub fn new(dmx_config: DmxBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: format!("DMX Bridge (Universe {})", dmx_config.universe),
            protocol: "dmx".to_string(),
            bidirectional: false, // DMX is output-only
            ..Default::default()
        };

        Self {
            config,
            dmx_config,
            running: Arc::new(Mutex::new(false)),
            tx: None,
            dmx_sender: None,
            dmx_state: Arc::new(Mutex::new([0u8; 512])),
            _output_thread: None,
        }
    }

    /// List available DMX interfaces
    pub fn list_ports() -> Result<Vec<String>> {
        // List available serial ports that might be DMX interfaces
        #[cfg(target_os = "macos")]
        {
            let ports: Vec<String> = std::fs::read_dir("/dev")
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path().to_string_lossy().to_string())
                        .filter(|p| p.contains("tty.usbserial") || p.contains("cu.usbserial"))
                        .collect()
                })
                .unwrap_or_default();
            Ok(ports)
        }

        #[cfg(target_os = "linux")]
        {
            let ports: Vec<String> = std::fs::read_dir("/dev")
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path().to_string_lossy().to_string())
                        .filter(|p| p.contains("ttyUSB") || p.contains("ttyACM"))
                        .collect()
                })
                .unwrap_or_default();
            Ok(ports)
        }

        #[cfg(target_os = "windows")]
        {
            // Windows COM ports
            Ok(vec![
                "COM1".to_string(),
                "COM2".to_string(),
                "COM3".to_string(),
            ])
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Ok(vec![])
        }
    }

    /// Set a single channel value
    pub fn set_channel(&self, channel: u16, value: u8) {
        if channel > 0 && channel <= 512 {
            let mut state = self.dmx_state.lock();
            state[(channel - 1) as usize] = value;

            if let Some(sender) = &self.dmx_sender {
                let _ = sender.tx.send(DmxCommand::SetChannel(channel, value));
            }
        }
    }

    /// Set entire DMX frame
    pub fn set_frame(&self, data: &[u8; 512]) {
        *self.dmx_state.lock() = *data;

        if let Some(sender) = &self.dmx_sender {
            let _ = sender.tx.send(DmxCommand::SetFrame(*data));
        }
    }

    /// Get current channel value
    pub fn get_channel(&self, channel: u16) -> Option<u8> {
        if channel > 0 && channel <= 512 {
            Some(self.dmx_state.lock()[(channel - 1) as usize])
        } else {
            None
        }
    }
}

#[async_trait]
impl Bridge for DmxBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let (tx, rx) = mpsc::channel(100);
        self.tx = Some(tx.clone());

        let (dmx_tx, dmx_rx) = std::sync::mpsc::channel::<DmxCommand>();
        self.dmx_sender = Some(DmxSender { tx: dmx_tx });

        let port_path = self.dmx_config.port.clone();
        let interface_type = self.dmx_config.interface_type;
        let refresh_rate = self.dmx_config.refresh_rate;
        let running = self.running.clone();
        let dmx_state = self.dmx_state.clone();

        // Spawn DMX output thread
        let output_thread = std::thread::spawn(move || {
            let refresh_interval =
                std::time::Duration::from_secs_f64(1.0 / refresh_rate);

            match interface_type {
                DmxInterfaceType::Virtual => {
                    info!("DMX bridge started in virtual mode");

                    // Virtual mode - just log changes
                    while *running.lock() {
                        match dmx_rx.recv_timeout(refresh_interval) {
                            Ok(DmxCommand::SetChannel(ch, val)) => {
                                debug!("Virtual DMX: Channel {} = {}", ch, val);
                            }
                            Ok(DmxCommand::SetFrame(_)) => {
                                debug!("Virtual DMX: Frame updated");
                            }
                            Ok(DmxCommand::Stop) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                // Send current frame at refresh rate
                                let _frame = *dmx_state.lock();
                                // In real implementation, send to hardware here
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                }
                DmxInterfaceType::EnttecPro => {
                    if let Some(port) = port_path {
                        info!("Opening ENTTEC DMX USB Pro on {}", port);
                        // TODO: Implement ENTTEC Pro protocol
                        // The ENTTEC Pro uses a specific serial protocol with:
                        // - Start code: 0x7E
                        // - Label (0x06 for DMX output)
                        // - Data length (2 bytes, LSB first)
                        // - DMX data (start code 0x00 + 512 channels)
                        // - End code: 0xE7
                        warn!("ENTTEC Pro not yet implemented, using virtual mode");
                    } else {
                        error!("No port specified for ENTTEC Pro");
                    }

                    // Fallback to virtual mode for now
                    while *running.lock() {
                        match dmx_rx.recv_timeout(refresh_interval) {
                            Ok(DmxCommand::Stop) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                            _ => {}
                        }
                    }
                }
                DmxInterfaceType::EnttecOpen | DmxInterfaceType::Ftdi => {
                    if let Some(port) = port_path {
                        info!("Opening DMX interface on {}", port);
                        // TODO: Implement FTDI-based DMX output
                        // This requires setting specific serial port parameters:
                        // - Baud rate: 250000
                        // - Break signal for frame start
                        // - 8N2 format
                        warn!("FTDI DMX not yet implemented, using virtual mode");
                    } else {
                        error!("No port specified for DMX interface");
                    }

                    while *running.lock() {
                        match dmx_rx.recv_timeout(refresh_interval) {
                            Ok(DmxCommand::Stop) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                            _ => {}
                        }
                    }
                }
            }

            info!("DMX output thread stopped");
        });

        self._output_thread = Some(output_thread);
        *self.running.lock() = true;

        let _ = tx.send(BridgeEvent::Connected).await;
        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;

        if let Some(sender) = &self.dmx_sender {
            let _ = sender.tx.send(DmxCommand::Stop);
        }

        self.tx = None;
        self.dmx_sender = None;
        self._output_thread = None;
        info!("DMX bridge stopped");
        Ok(())
    }

    async fn send(&self, message: Message) -> Result<()> {
        match &message {
            Message::Set(set) => {
                // Parse address: /dmx/{universe}/{channel}
                let parts: Vec<&str> = set.address.split('/').collect();

                if parts.len() >= 4 {
                    let universe: u16 = parts[2]
                        .parse()
                        .map_err(|_| BridgeError::Mapping("Invalid universe".to_string()))?;

                    // Check if this is our universe
                    if universe != self.dmx_config.universe {
                        return Ok(());
                    }

                    let channel: u16 = parts[3]
                        .parse()
                        .map_err(|_| BridgeError::Mapping("Invalid channel".to_string()))?;

                    if channel > 0 && channel <= 512 {
                        let value = set.value.as_i64().unwrap_or(0).clamp(0, 255) as u8;
                        self.set_channel(channel, value);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.dmx_config.namespace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DmxBridgeConfig::default();
        assert_eq!(config.namespace, "/dmx");
        assert_eq!(config.universe, 0);
        assert_eq!(config.interface_type, DmxInterfaceType::Virtual);
    }

    #[test]
    fn test_channel_operations() {
        let bridge = DmxBridge::new(DmxBridgeConfig::default());

        // Test set/get channel
        {
            let mut state = bridge.dmx_state.lock();
            state[0] = 127;
        }
        assert_eq!(bridge.get_channel(1), Some(127));

        // Invalid channel
        assert_eq!(bridge.get_channel(0), None);
        assert_eq!(bridge.get_channel(513), None);
    }
}
