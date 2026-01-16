//! OSC (Open Sound Control) bridge

use async_trait::async_trait;
use parking_lot::Mutex;
use rosc::{OscMessage, OscPacket, OscType};
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};

/// OSC bridge configuration
#[derive(Debug, Clone)]
pub struct OscBridgeConfig {
    /// Local address to bind
    pub bind_addr: String,
    /// Remote address to send to (optional)
    pub remote_addr: Option<String>,
    /// Address prefix for Clasp
    pub namespace: String,
}

impl Default for OscBridgeConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8000".to_string(),
            remote_addr: None,
            namespace: "/osc".to_string(),
        }
    }
}

/// OSC to Clasp bridge
pub struct OscBridge {
    config: BridgeConfig,
    osc_config: OscBridgeConfig,
    socket: Option<Arc<UdpSocket>>,
    running: Arc<Mutex<bool>>,
}

impl OscBridge {
    pub fn new(osc_config: OscBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: "OSC Bridge".to_string(),
            protocol: "osc".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            osc_config,
            socket: None,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Convert OSC message to Clasp message
    fn osc_to_clasp(&self, msg: &OscMessage) -> Option<Message> {
        let address = format!("{}{}", self.osc_config.namespace, msg.addr);

        // Convert OSC args to Clasp value
        let value = if msg.args.is_empty() {
            Value::Null
        } else if msg.args.len() == 1 {
            osc_arg_to_value(&msg.args[0])
        } else {
            Value::Array(msg.args.iter().map(osc_arg_to_value).collect())
        };

        Some(Message::Set(SetMessage {
            address,
            value,
            revision: None,
            lock: false,
            unlock: false,
        }))
    }

    /// Convert Clasp message to OSC
    fn clasp_to_osc(&self, msg: &Message) -> Option<OscPacket> {
        match msg {
            Message::Set(set) => {
                // Strip namespace prefix
                let addr = set
                    .address
                    .strip_prefix(&self.osc_config.namespace)
                    .unwrap_or(&set.address);

                let args = value_to_osc_args(&set.value);

                Some(OscPacket::Message(OscMessage {
                    addr: addr.to_string(),
                    args,
                }))
            }
            Message::Publish(pub_msg) => {
                let addr = pub_msg
                    .address
                    .strip_prefix(&self.osc_config.namespace)
                    .unwrap_or(&pub_msg.address);

                let args = if let Some(ref value) = pub_msg.value {
                    value_to_osc_args(value)
                } else if let Some(ref payload) = pub_msg.payload {
                    value_to_osc_args(payload)
                } else {
                    vec![]
                };

                Some(OscPacket::Message(OscMessage {
                    addr: addr.to_string(),
                    args,
                }))
            }
            _ => None,
        }
    }
}

#[async_trait]
impl Bridge for OscBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let socket = UdpSocket::bind(&self.osc_config.bind_addr)
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        info!("OSC bridge listening on {}", self.osc_config.bind_addr);

        let socket = Arc::new(socket);
        self.socket = Some(socket.clone());
        *self.running.lock() = true;

        let (tx, rx) = mpsc::channel(100);
        let running = self.running.clone();
        let namespace = self.osc_config.namespace.clone();

        // Spawn receiver task
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];

            let _ = tx.send(BridgeEvent::Connected).await;

            while *running.lock() {
                match socket.recv_from(&mut buf).await {
                    Ok((len, from)) => {
                        debug!("OSC received {} bytes from {}", len, from);

                        // Parse OSC packet
                        match rosc::decoder::decode_udp(&buf[..len]) {
                            Ok((_, packet)) => {
                                if let Some(messages) = packet_to_messages(&packet, &namespace) {
                                    for msg in messages {
                                        if tx.send(BridgeEvent::ToClasp(msg)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("OSC decode error: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("OSC receive error: {}", e);
                        let _ = tx.send(BridgeEvent::Error(e.to_string())).await;
                    }
                }
            }

            let _ = tx
                .send(BridgeEvent::Disconnected { reason: None })
                .await;
        });

        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;
        self.socket = None;
        info!("OSC bridge stopped");
        Ok(())
    }

    async fn send(&self, message: Message) -> Result<()> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| BridgeError::ConnectionFailed("Not connected".to_string()))?;

        let remote = self
            .osc_config
            .remote_addr
            .as_ref()
            .ok_or_else(|| BridgeError::Send("No remote address configured".to_string()))?;

        let remote_addr: SocketAddr = remote
            .parse()
            .map_err(|e| BridgeError::Send(format!("Invalid remote address: {}", e)))?;

        if let Some(packet) = self.clasp_to_osc(&message) {
            let bytes = rosc::encoder::encode(&packet)
                .map_err(|e| BridgeError::Protocol(format!("OSC encode error: {:?}", e)))?;

            socket
                .send_to(&bytes, remote_addr)
                .await
                .map_err(|e| BridgeError::Send(e.to_string()))?;

            debug!("Sent OSC message to {}", remote_addr);
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.osc_config.namespace
    }
}

/// Convert OSC argument to Clasp value
fn osc_arg_to_value(arg: &OscType) -> Value {
    match arg {
        OscType::Int(i) => Value::Int(*i as i64),
        OscType::Float(f) => Value::Float(*f as f64),
        OscType::String(s) => Value::String(s.clone()),
        OscType::Blob(b) => Value::Bytes(b.clone()),
        OscType::Long(l) => Value::Int(*l),
        OscType::Double(d) => Value::Float(*d),
        OscType::Bool(b) => Value::Bool(*b),
        OscType::Nil => Value::Null,
        OscType::Inf => Value::Float(f64::INFINITY),
        _ => Value::Null,
    }
}

/// Convert Clasp value to OSC arguments
fn value_to_osc_args(value: &Value) -> Vec<OscType> {
    match value {
        Value::Null => vec![],
        Value::Bool(b) => vec![OscType::Bool(*b)],
        Value::Int(i) => vec![OscType::Long(*i)],
        Value::Float(f) => vec![OscType::Double(*f)],
        Value::String(s) => vec![OscType::String(s.clone())],
        Value::Bytes(b) => vec![OscType::Blob(b.clone())],
        Value::Array(arr) => arr.iter().flat_map(value_to_osc_args).collect(),
        Value::Map(_) => vec![OscType::String(serde_json::to_string(value).unwrap_or_default())],
    }
}

/// Convert OSC packet to Clasp messages
fn packet_to_messages(packet: &OscPacket, namespace: &str) -> Option<Vec<Message>> {
    match packet {
        OscPacket::Message(msg) => {
            let address = format!("{}{}", namespace, msg.addr);
            let value = if msg.args.is_empty() {
                Value::Null
            } else if msg.args.len() == 1 {
                osc_arg_to_value(&msg.args[0])
            } else {
                Value::Array(msg.args.iter().map(osc_arg_to_value).collect())
            };

            Some(vec![Message::Set(SetMessage {
                address,
                value,
                revision: None,
                lock: false,
                unlock: false,
            })])
        }
        OscPacket::Bundle(bundle) => {
            let messages: Vec<Message> = bundle
                .content
                .iter()
                .filter_map(|p| packet_to_messages(p, namespace))
                .flatten()
                .collect();

            if messages.is_empty() {
                None
            } else {
                // Wrap in bundle with timestamp
                let timestamp = bundle.timetag.seconds as u64 * 1_000_000
                    + (bundle.timetag.fractional as u64 * 1_000_000 / u32::MAX as u64);

                Some(vec![Message::Bundle(clasp_core::BundleMessage {
                    timestamp: Some(timestamp),
                    messages,
                })])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_arg_conversion() {
        assert_eq!(osc_arg_to_value(&OscType::Int(42)), Value::Int(42));
        assert_eq!(osc_arg_to_value(&OscType::Float(0.5)), Value::Float(0.5));
        assert_eq!(
            osc_arg_to_value(&OscType::String("test".to_string())),
            Value::String("test".to_string())
        );
    }

    #[test]
    fn test_value_to_osc() {
        let args = value_to_osc_args(&Value::Float(0.75));
        assert_eq!(args.len(), 1);
        match &args[0] {
            OscType::Double(f) => assert!((f - 0.75).abs() < 0.001),
            _ => panic!("Expected Double"),
        }
    }
}
