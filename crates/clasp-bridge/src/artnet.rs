//! Art-Net bridge

use async_trait::async_trait;
use artnet_protocol::{ArtCommand, Output, Poll};
use parking_lot::Mutex;
use clasp_core::{Message, SetMessage, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};

/// Art-Net port
const ARTNET_PORT: u16 = 6454;

/// Art-Net bridge configuration
#[derive(Debug, Clone)]
pub struct ArtNetBridgeConfig {
    /// Local address to bind
    pub bind_addr: String,
    /// Remote Art-Net node (for output)
    pub remote_addr: Option<String>,
    /// Universes to listen to (empty = all)
    pub universes: Vec<u16>,
    /// Address namespace
    pub namespace: String,
}

impl Default for ArtNetBridgeConfig {
    fn default() -> Self {
        Self {
            bind_addr: format!("0.0.0.0:{}", ARTNET_PORT),
            remote_addr: None,
            universes: vec![],
            namespace: "/artnet".to_string(),
        }
    }
}

/// Art-Net to SignalFlow bridge
pub struct ArtNetBridge {
    config: BridgeConfig,
    artnet_config: ArtNetBridgeConfig,
    socket: Option<Arc<UdpSocket>>,
    running: Arc<Mutex<bool>>,
    /// Current DMX values per universe (for delta detection)
    dmx_state: Arc<Mutex<std::collections::HashMap<u16, [u8; 512]>>>,
}

impl ArtNetBridge {
    pub fn new(artnet_config: ArtNetBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: "Art-Net Bridge".to_string(),
            protocol: "artnet".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            artnet_config,
            socket: None,
            running: Arc::new(Mutex::new(false)),
            dmx_state: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Send Art-Net poll to discover nodes
    pub async fn poll(&self) -> Result<()> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| BridgeError::ConnectionFailed("Not connected".to_string()))?;

        let poll = ArtCommand::Poll(Poll::default());
        let bytes = poll
            .into_buffer()
            .map_err(|e| BridgeError::Protocol(format!("Failed to encode poll: {:?}", e)))?;

        // Broadcast poll
        let broadcast = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), ARTNET_PORT);
        socket
            .send_to(&bytes, broadcast)
            .await
            .map_err(|e| BridgeError::Send(e.to_string()))?;

        debug!("Sent Art-Net poll");
        Ok(())
    }

    /// Send DMX data to a universe
    pub async fn send_dmx(&self, universe: u16, data: &[u8]) -> Result<()> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| BridgeError::ConnectionFailed("Not connected".to_string()))?;

        let remote = self
            .artnet_config
            .remote_addr
            .as_ref()
            .ok_or_else(|| BridgeError::Send("No remote address configured".to_string()))?;

        let remote_addr: SocketAddr = remote
            .parse()
            .map_err(|e| BridgeError::Send(format!("Invalid remote address: {}", e)))?;

        // Create DMX output command
        // In artnet_protocol 0.2, Output has: version, sequence, physical, subnet, length, data
        // The subnet field is used for the universe/subnet addressing
        let mut output = Output::default();
        output.subnet = universe; // Universe goes in subnet field
        output.data = data.to_vec().into();
        output.length = data.len() as u16;

        let command = ArtCommand::Output(output);
        let bytes = command
            .into_buffer()
            .map_err(|e| BridgeError::Protocol(format!("Failed to encode DMX: {:?}", e)))?;

        socket
            .send_to(&bytes, remote_addr)
            .await
            .map_err(|e| BridgeError::Send(e.to_string()))?;

        debug!("Sent DMX to universe {} ({} bytes)", universe, data.len());
        Ok(())
    }
}

#[async_trait]
impl Bridge for ArtNetBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let socket = UdpSocket::bind(&self.artnet_config.bind_addr)
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        // Enable broadcast for poll
        socket
            .set_broadcast(true)
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        info!(
            "Art-Net bridge listening on {}",
            self.artnet_config.bind_addr
        );

        let socket = Arc::new(socket);
        self.socket = Some(socket.clone());
        *self.running.lock() = true;

        let (tx, rx) = mpsc::channel(100);
        let running = self.running.clone();
        let namespace = self.artnet_config.namespace.clone();
        let universes = self.artnet_config.universes.clone();
        let dmx_state = self.dmx_state.clone();

        // Spawn receiver task
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];

            let _ = tx.send(BridgeEvent::Connected).await;

            while *running.lock() {
                match socket.recv_from(&mut buf).await {
                    Ok((len, from)) => {
                        // Parse Art-Net packet
                        match ArtCommand::from_buffer(&buf[..len]) {
                            Ok(command) => {
                                if let Some(messages) =
                                    artnet_to_clasp(&command, &namespace, &universes, &dmx_state)
                                {
                                    for msg in messages {
                                        if tx.send(BridgeEvent::ToSignalFlow(msg)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Art-Net decode error from {}: {:?}", from, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Art-Net receive error: {}", e);
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
        info!("Art-Net bridge stopped");
        Ok(())
    }

    async fn send(&self, message: Message) -> Result<()> {
        match &message {
            Message::Set(set) => {
                // Parse address: /artnet/{universe}/{channel}
                let parts: Vec<&str> = set.address.split('/').collect();

                if parts.len() >= 4 {
                    let universe: u16 = parts[2]
                        .parse()
                        .map_err(|_| BridgeError::Mapping("Invalid universe".to_string()))?;
                    let channel: usize = parts[3]
                        .parse()
                        .map_err(|_| BridgeError::Mapping("Invalid channel".to_string()))?;

                    if channel > 0 && channel <= 512 {
                        let value = set.value.as_i64().unwrap_or(0).clamp(0, 255) as u8;

                        // Get the DMX data, update it, then release the lock before await
                        let dmx_copy = {
                            let mut state = self.dmx_state.lock();
                            let dmx = state.entry(universe).or_insert([0u8; 512]);
                            dmx[channel - 1] = value;
                            *dmx // Copy the array
                        };

                        // Now send without holding the lock
                        self.send_dmx(universe, &dmx_copy).await?;
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
        &self.artnet_config.namespace
    }
}

/// Convert Art-Net command to SignalFlow messages
fn artnet_to_clasp(
    command: &ArtCommand,
    namespace: &str,
    filter_universes: &[u16],
    dmx_state: &Arc<Mutex<std::collections::HashMap<u16, [u8; 512]>>>,
) -> Option<Vec<Message>> {
    match command {
        ArtCommand::Output(output) => {
            // In artnet_protocol 0.2, use subnet field for universe
            let universe = output.subnet;

            // Check universe filter
            if !filter_universes.is_empty() && !filter_universes.contains(&universe) {
                return None;
            }

            let data: &[u8] = &output.data;

            // Compare with previous state to only send changes
            let mut state = dmx_state.lock();
            let prev = state.entry(universe).or_insert([0u8; 512]);

            let mut messages = Vec::new();

            for (i, &value) in data.iter().enumerate() {
                if value != prev[i] {
                    prev[i] = value;

                    messages.push(Message::Set(SetMessage {
                        address: format!("{}/{}/{}", namespace, universe, i + 1),
                        value: Value::Int(value as i64),
                        revision: None,
                        lock: false,
                        unlock: false,
                    }));
                }
            }

            if messages.is_empty() {
                None
            } else {
                Some(messages)
            }
        }
        ArtCommand::Poll(_) => {
            debug!("Received Art-Net Poll");
            None
        }
        ArtCommand::PollReply(reply) => {
            debug!("Received Art-Net PollReply from {:?}", reply.address);
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ArtNetBridgeConfig::default();
        assert_eq!(config.namespace, "/artnet");
        assert!(config.universes.is_empty());
    }
}
