//! MIDI bridge

use async_trait::async_trait;
use midir::{MidiInput, MidiInputPort, MidiOutput, MidiOutputPort};
use parking_lot::Mutex;
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};

/// MIDI bridge configuration
#[derive(Debug, Clone)]
pub struct MidiBridgeConfig {
    /// Input port name (or index)
    pub input_port: Option<String>,
    /// Output port name (or index)
    pub output_port: Option<String>,
    /// Address namespace
    pub namespace: String,
    /// Device name in addresses
    pub device_name: String,
}

impl Default for MidiBridgeConfig {
    fn default() -> Self {
        Self {
            input_port: None,
            output_port: None,
            namespace: "/midi".to_string(),
            device_name: "default".to_string(),
        }
    }
}

/// MIDI output sender for thread-safe sending
struct MidiSender {
    tx: std::sync::mpsc::Sender<Vec<u8>>,
}

impl MidiSender {
    fn send(&self, data: Vec<u8>) -> std::result::Result<(), String> {
        self.tx.send(data).map_err(|e| e.to_string())
    }
}

/// MIDI to SignalFlow bridge
pub struct MidiBridge {
    config: BridgeConfig,
    midi_config: MidiBridgeConfig,
    running: Arc<Mutex<bool>>,
    tx: Option<mpsc::Sender<BridgeEvent>>,
    /// Thread-safe sender for MIDI output
    midi_sender: Option<MidiSender>,
    /// Handle to keep input connection alive
    _input_thread: Option<std::thread::JoinHandle<()>>,
    /// Handle to keep output connection alive
    _output_thread: Option<std::thread::JoinHandle<()>>,
}

impl MidiBridge {
    pub fn new(midi_config: MidiBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: format!("MIDI Bridge ({})", midi_config.device_name),
            protocol: "midi".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            midi_config,
            running: Arc::new(Mutex::new(false)),
            tx: None,
            midi_sender: None,
            _input_thread: None,
            _output_thread: None,
        }
    }

    /// List available MIDI input ports
    pub fn list_input_ports() -> Result<Vec<String>> {
        let midi_in = MidiInput::new("SignalFlow MIDI Scanner")
            .map_err(|e| BridgeError::Protocol(e.to_string()))?;

        let ports = midi_in.ports();
        Ok(ports
            .iter()
            .filter_map(|p| midi_in.port_name(p).ok())
            .collect())
    }

    /// List available MIDI output ports
    pub fn list_output_ports() -> Result<Vec<String>> {
        let midi_out = MidiOutput::new("SignalFlow MIDI Scanner")
            .map_err(|e| BridgeError::Protocol(e.to_string()))?;

        let ports = midi_out.ports();
        Ok(ports
            .iter()
            .filter_map(|p| midi_out.port_name(p).ok())
            .collect())
    }

    /// Find input port by name or use first available
    fn find_input_port(midi_in: &MidiInput, port_name: Option<&str>) -> Option<MidiInputPort> {
        let ports = midi_in.ports();
        if ports.is_empty() {
            return None;
        }

        if let Some(name) = port_name {
            ports.into_iter().find(|p| {
                midi_in
                    .port_name(p)
                    .map(|n| n.contains(name))
                    .unwrap_or(false)
            })
        } else {
            ports.into_iter().next()
        }
    }

    /// Find output port by name or use first available
    fn find_output_port(midi_out: &MidiOutput, port_name: Option<&str>) -> Option<MidiOutputPort> {
        let ports = midi_out.ports();
        if ports.is_empty() {
            return None;
        }

        if let Some(name) = port_name {
            ports.into_iter().find(|p| {
                midi_out
                    .port_name(p)
                    .map(|n| n.contains(name))
                    .unwrap_or(false)
            })
        } else {
            ports.into_iter().next()
        }
    }
}

#[async_trait]
impl Bridge for MidiBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let (tx, rx) = mpsc::channel(100);
        self.tx = Some(tx.clone());

        // Set up MIDI input in a separate thread (midir types are not Send)
        let namespace = self.midi_config.namespace.clone();
        let device_name = self.midi_config.device_name.clone();
        let input_port_name = self.midi_config.input_port.clone();
        let running = self.running.clone();

        let input_thread = std::thread::spawn(move || {
            let midi_in = match MidiInput::new("SignalFlow MIDI Input") {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to create MIDI input: {}", e);
                    return;
                }
            };

            let port = match Self::find_input_port(&midi_in, input_port_name.as_deref()) {
                Some(p) => p,
                None => {
                    warn!("No MIDI input port found");
                    return;
                }
            };

            let port_name = midi_in.port_name(&port).unwrap_or_else(|_| "Unknown".to_string());
            info!("Opening MIDI input: {}", port_name);

            let tx_clone = tx.clone();
            let base_addr = format!("{}/{}", namespace, device_name);

            // Connect to input - this blocks until disconnected
            let _conn = match midi_in.connect(
                &port,
                "clasp-midi",
                move |_stamp, message, _| {
                    if let Some(msg) = midi_message_to_clasp(message, &base_addr) {
                        // Use blocking send since we're in a callback
                        let tx = tx_clone.clone();
                        // Spawn a task to send asynchronously
                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Handle::try_current();
                            if let Ok(handle) = rt {
                                handle.spawn(async move {
                                    let _ = tx.send(BridgeEvent::ToSignalFlow(msg)).await;
                                });
                            }
                        });
                    }
                },
                (),
            ) {
                Ok(conn) => conn,
                Err(e) => {
                    warn!("Failed to connect to MIDI input: {}", e);
                    return;
                }
            };

            // Keep thread alive while running
            while *running.lock() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        self._input_thread = Some(input_thread);

        // Set up MIDI output in a separate thread
        let output_port_name = self.midi_config.output_port.clone();
        let running_out = self.running.clone();
        let (midi_tx, midi_rx) = std::sync::mpsc::channel::<Vec<u8>>();

        let output_thread = std::thread::spawn(move || {
            let midi_out = match MidiOutput::new("SignalFlow MIDI Output") {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to create MIDI output: {}", e);
                    return;
                }
            };

            let port = match Self::find_output_port(&midi_out, output_port_name.as_deref()) {
                Some(p) => p,
                None => {
                    warn!("No MIDI output port found");
                    return;
                }
            };

            let port_name = midi_out.port_name(&port).unwrap_or_else(|_| "Unknown".to_string());
            info!("Opening MIDI output: {}", port_name);

            let mut conn = match midi_out.connect(&port, "clasp-midi") {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to connect to MIDI output: {}", e);
                    return;
                }
            };

            info!("MIDI output connected");

            // Process outgoing MIDI messages
            while *running_out.lock() {
                match midi_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(data) => {
                        if let Err(e) = conn.send(&data) {
                            warn!("MIDI send error: {}", e);
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        self._output_thread = Some(output_thread);
        self.midi_sender = Some(MidiSender { tx: midi_tx });

        *self.running.lock() = true;
        let _ = self
            .tx
            .as_ref()
            .unwrap()
            .send(BridgeEvent::Connected)
            .await;

        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;
        self.tx = None;
        self.midi_sender = None;
        // Threads will exit on their own when running becomes false
        self._input_thread = None;
        self._output_thread = None;
        info!("MIDI bridge stopped");
        Ok(())
    }

    async fn send(&self, message: Message) -> Result<()> {
        let sender = self
            .midi_sender
            .as_ref()
            .ok_or_else(|| BridgeError::Send("No MIDI output connected".to_string()))?;

        // Convert SignalFlow to MIDI
        if let Some(midi_msg) = clasp_to_midi(&message, &self.midi_config) {
            sender
                .send(midi_msg)
                .map_err(|e| BridgeError::Send(e.to_string()))?;
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.midi_config.namespace
    }
}

/// Convert MIDI message bytes to SignalFlow (standalone function for callback)
fn midi_message_to_clasp(message: &[u8], base_addr: &str) -> Option<Message> {
    if message.is_empty() {
        return None;
    }

    let status = message[0] & 0xF0;
    let channel = message[0] & 0x0F;
    let addr = format!("{}/ch/{}", base_addr, channel);

    match status {
        // Note Off / Note On
        0x80 | 0x90 => {
            let note = message.get(1).copied().unwrap_or(0) as i64;
            let velocity = message.get(2).copied().unwrap_or(0) as i64;
            let on = status == 0x90 && velocity > 0;
            Some(Message::Publish(PublishMessage {
                address: format!("{}/note", addr),
                signal: Some(SignalType::Event),
                value: None,
                payload: Some(Value::Map(
                    [
                        ("note".to_string(), Value::Int(note)),
                        ("velocity".to_string(), Value::Int(velocity)),
                        ("on".to_string(), Value::Bool(on)),
                    ]
                    .into_iter()
                    .collect(),
                )),
                samples: None,
                rate: None,
                id: None,
                phase: None,
                timestamp: None,
            }))
        }
        // Control Change
        0xB0 => {
            let cc = message.get(1).copied().unwrap_or(0);
            let value = message.get(2).copied().unwrap_or(0) as i64;
            Some(Message::Set(SetMessage {
                address: format!("{}/cc/{}", addr, cc),
                value: Value::Int(value),
                revision: None,
                lock: false,
                unlock: false,
            }))
        }
        // Program Change
        0xC0 => {
            let program = message.get(1).copied().unwrap_or(0) as i64;
            Some(Message::Publish(PublishMessage {
                address: format!("{}/program", addr),
                signal: Some(SignalType::Event),
                value: None,
                payload: Some(Value::Int(program)),
                samples: None,
                rate: None,
                id: None,
                phase: None,
                timestamp: None,
            }))
        }
        // Pitch Bend
        0xE0 => {
            let lsb = message.get(1).copied().unwrap_or(0) as i64;
            let msb = message.get(2).copied().unwrap_or(0) as i64;
            let value = ((msb << 7) | lsb) - 8192;
            Some(Message::Set(SetMessage {
                address: format!("{}/bend", addr),
                value: Value::Int(value),
                revision: None,
                lock: false,
                unlock: false,
            }))
        }
        // System messages (clock, transport)
        0xF0 => match message[0] {
            0xF8 => Some(Message::Publish(PublishMessage {
                address: format!("{}/clock", base_addr),
                signal: Some(SignalType::Event),
                value: None,
                payload: None,
                samples: None,
                rate: None,
                id: None,
                phase: None,
                timestamp: None,
            })),
            0xFA => Some(Message::Publish(PublishMessage {
                address: format!("{}/transport", base_addr),
                signal: Some(SignalType::Event),
                value: None,
                payload: Some(Value::String("start".to_string())),
                samples: None,
                rate: None,
                id: None,
                phase: None,
                timestamp: None,
            })),
            0xFB => Some(Message::Publish(PublishMessage {
                address: format!("{}/transport", base_addr),
                signal: Some(SignalType::Event),
                value: None,
                payload: Some(Value::String("continue".to_string())),
                samples: None,
                rate: None,
                id: None,
                phase: None,
                timestamp: None,
            })),
            0xFC => Some(Message::Publish(PublishMessage {
                address: format!("{}/transport", base_addr),
                signal: Some(SignalType::Event),
                value: None,
                payload: Some(Value::String("stop".to_string())),
                samples: None,
                rate: None,
                id: None,
                phase: None,
                timestamp: None,
            })),
            _ => None,
        },
        _ => None,
    }
}

/// Convert SignalFlow message to MIDI bytes
fn clasp_to_midi(message: &Message, _config: &MidiBridgeConfig) -> Option<Vec<u8>> {
    match message {
        Message::Set(set) => {
            // Parse address to extract MIDI parameters
            let parts: Vec<&str> = set.address.split('/').collect();

            // Looking for pattern: /midi/{device}/ch/{channel}/cc/{num}
            if parts.len() >= 6 && parts[4] == "cc" {
                let channel: u8 = parts[3].parse().ok()?;
                let cc: u8 = parts[5].parse().ok()?;
                let value = set.value.as_i64()?.clamp(0, 127) as u8;

                return Some(vec![0xB0 | (channel & 0x0F), cc, value]);
            }

            // Looking for pattern: /midi/{device}/ch/{channel}/bend
            if parts.len() >= 5 && parts[4] == "bend" {
                let channel: u8 = parts[3].parse().ok()?;
                let value = (set.value.as_i64()? + 8192).clamp(0, 16383) as u16;
                let lsb = (value & 0x7F) as u8;
                let msb = ((value >> 7) & 0x7F) as u8;

                return Some(vec![0xE0 | (channel & 0x0F), lsb, msb]);
            }

            None
        }
        Message::Publish(pub_msg) => {
            // Handle note events
            let parts: Vec<&str> = pub_msg.address.split('/').collect();

            // Looking for pattern: /midi/{device}/ch/{channel}/note
            if parts.len() >= 5 && parts[4] == "note" {
                let channel: u8 = parts[3].parse().ok()?;

                if let Some(Value::Map(map)) = &pub_msg.payload {
                    let note = map.get("note")?.as_i64()?.clamp(0, 127) as u8;
                    let velocity = map.get("velocity")?.as_i64()?.clamp(0, 127) as u8;
                    let on = map.get("on").and_then(|v| v.as_bool()).unwrap_or(velocity > 0);

                    let status = if on { 0x90 } else { 0x80 };
                    return Some(vec![status | (channel & 0x0F), note, velocity]);
                }
            }

            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_to_clasp_note_on() {
        let msg = midi_message_to_clasp(&[0x90, 60, 100], "/midi/test");
        assert!(msg.is_some());
        if let Some(Message::Publish(pub_msg)) = msg {
            assert_eq!(pub_msg.address, "/midi/test/ch/0/note");
        }
    }

    #[test]
    fn test_midi_to_clasp_cc() {
        let msg = midi_message_to_clasp(&[0xB0, 1, 64], "/midi/test");
        assert!(msg.is_some());
        if let Some(Message::Set(set_msg)) = msg {
            assert_eq!(set_msg.address, "/midi/test/ch/0/cc/1");
            assert_eq!(set_msg.value, Value::Int(64));
        }
    }

    #[test]
    fn test_config_default() {
        let config = MidiBridgeConfig::default();
        assert_eq!(config.namespace, "/midi");
        assert_eq!(config.device_name, "default");
    }
}
