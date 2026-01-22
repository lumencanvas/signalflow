//! sACN/E1.31 (Streaming ACN) bridge
//!
//! Implements the ANSI E1.31 protocol for streaming DMX512-A data over IP networks.
//! This is the standard for professional lighting installations, supporting:
//! - Multicast discovery and data transmission
//! - Priority-based source selection
//! - Synchronization between universes
//!
//! # Example
//!
//! ```no_run
//! use clasp_bridge::sacn::{SacnBridge, SacnBridgeConfig, SacnMode};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = SacnBridgeConfig {
//!         mode: SacnMode::Receiver,
//!         universes: vec![1, 2, 3],
//!         priority: 100,
//!         ..Default::default()
//!     };
//!
//!     let mut bridge = SacnBridge::new(config);
//!     let mut events = bridge.start().await.unwrap();
//!
//!     while let Some(event) = events.recv().await {
//!         println!("sACN event: {:?}", event);
//!     }
//! }
//! ```

use async_trait::async_trait;
use clasp_core::{Message, SetMessage, Value};
use parking_lot::Mutex;
use sacn_lib::packet::ACN_SDT_MULTICAST_PORT;
use sacn_lib::receive::SacnReceiver;
use sacn_lib::source::SacnSource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{Bridge, BridgeConfig as TraitBridgeConfig, BridgeError, BridgeEvent, Result};

/// sACN operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SacnMode {
    /// Send DMX data to the network
    Sender,
    /// Receive DMX data from the network
    #[default]
    Receiver,
    /// Both send and receive (bidirectional)
    Bidirectional,
}

/// sACN bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SacnBridgeConfig {
    /// Operating mode
    #[serde(default)]
    pub mode: SacnMode,
    /// Universes to send/receive
    #[serde(default = "default_universes")]
    pub universes: Vec<u16>,
    /// Source name (for sender mode)
    #[serde(default = "default_source_name")]
    pub source_name: String,
    /// Priority (0-200, higher = more important)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Network interface to bind to (None = all interfaces)
    #[serde(default)]
    pub bind_address: Option<String>,
    /// Use multicast (true) or unicast (false)
    #[serde(default = "default_multicast")]
    pub multicast: bool,
    /// Unicast destination addresses (only used if multicast = false)
    #[serde(default)]
    pub unicast_destinations: Vec<String>,
    /// CLASP address namespace prefix
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// Preview data only (won't affect output)
    #[serde(default)]
    pub preview: bool,
    /// Synchronization address (0 = no sync)
    #[serde(default)]
    pub sync_address: u16,
}

fn default_universes() -> Vec<u16> {
    vec![1]
}

fn default_source_name() -> String {
    "CLASP sACN Bridge".to_string()
}

fn default_priority() -> u8 {
    100 // Middle priority
}

fn default_multicast() -> bool {
    true
}

fn default_namespace() -> String {
    "/sacn".to_string()
}

impl Default for SacnBridgeConfig {
    fn default() -> Self {
        Self {
            mode: SacnMode::Receiver,
            universes: default_universes(),
            source_name: default_source_name(),
            priority: default_priority(),
            bind_address: None,
            multicast: true,
            unicast_destinations: vec![],
            namespace: default_namespace(),
            preview: false,
            sync_address: 0,
        }
    }
}

/// sACN/E1.31 bridge
pub struct SacnBridge {
    config: TraitBridgeConfig,
    sacn_config: SacnBridgeConfig,
    running: Arc<Mutex<bool>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// DMX data cache for sender mode (universe -> channel data)
    dmx_data: Arc<Mutex<HashMap<u16, [u8; 512]>>>,
    /// Channel for sending DMX data updates
    send_tx: Option<mpsc::Sender<(u16, u16, u8)>>, // (universe, channel, value)
}

impl SacnBridge {
    /// Create a new sACN bridge
    pub fn new(config: SacnBridgeConfig) -> Self {
        let bridge_config = TraitBridgeConfig {
            name: config.source_name.clone(),
            protocol: "sacn".to_string(),
            bidirectional: config.mode == SacnMode::Bidirectional,
            options: HashMap::new(),
        };

        // Initialize DMX data cache
        let mut dmx_data = HashMap::new();
        for &universe in &config.universes {
            dmx_data.insert(universe, [0u8; 512]);
        }

        Self {
            config: bridge_config,
            sacn_config: config,
            running: Arc::new(Mutex::new(false)),
            shutdown_tx: None,
            dmx_data: Arc::new(Mutex::new(dmx_data)),
            send_tx: None,
        }
    }

    /// Convert sACN data to CLASP message
    fn to_clasp_message(namespace: &str, universe: u16, channel: u16, value: u8) -> Message {
        Message::Set(SetMessage {
            address: format!("{}/{}/{}", namespace, universe, channel),
            value: Value::Int(value as i64),
            revision: None,
            lock: false,
            unlock: false,
        })
    }

    /// Convert CLASP address to sACN universe/channel
    fn parse_address(namespace: &str, address: &str) -> Option<(u16, u16)> {
        let stripped = address.strip_prefix(namespace)?;
        let parts: Vec<&str> = stripped.trim_start_matches('/').split('/').collect();
        if parts.len() == 2 {
            let universe: u16 = parts[0].parse().ok()?;
            let channel: u16 = parts[1].parse().ok()?;
            if channel >= 1 && channel <= 512 {
                Some((universe, channel))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Run receiver mode
    async fn run_receiver(
        config: SacnBridgeConfig,
        event_tx: mpsc::Sender<BridgeEvent>,
        mut shutdown_rx: mpsc::Receiver<()>,
        running: Arc<Mutex<bool>>,
    ) {
        let bind_addr: SocketAddr = config
            .bind_address
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), ACN_SDT_MULTICAST_PORT)
            });

        // Create receiver
        let mut receiver = match SacnReceiver::with_ip(bind_addr, None) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to create sACN receiver: {}", e);
                let _ = event_tx
                    .send(BridgeEvent::Error(format!("sACN receiver error: {}", e)))
                    .await;
                return;
            }
        };

        // Subscribe to universes
        for &universe in &config.universes {
            if let Err(e) = receiver.listen_universes(&[universe]) {
                warn!("Failed to subscribe to universe {}: {}", universe, e);
            } else {
                info!("sACN subscribed to universe {}", universe);
            }
        }

        *running.lock() = true;
        let _ = event_tx.send(BridgeEvent::Connected).await;
        info!(
            "sACN receiver started on {:?}, universes: {:?}",
            bind_addr, config.universes
        );

        // Track previous values to only send changes
        let mut prev_values: HashMap<(u16, u16), u8> = HashMap::new();

        loop {
            tokio::select! {
                // Check for shutdown
                _ = shutdown_rx.recv() => {
                    info!("sACN receiver shutting down");
                    break;
                }
                // Poll for data (non-blocking with timeout)
                _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {
                    // Try to receive data
                    match receiver.recv(Some(std::time::Duration::from_millis(1))) {
                        Ok(packets) => {
                            for packet in packets {
                                let universe = packet.universe;
                                let data = packet.values;

                                // Send changes to CLASP
                                for (idx, &value) in data.iter().enumerate() {
                                    let channel = (idx + 1) as u16;
                                    let key = (universe, channel);

                                    // Only send if value changed
                                    if prev_values.get(&key) != Some(&value) {
                                        prev_values.insert(key, value);

                                        let msg = Self::to_clasp_message(
                                            &config.namespace,
                                            universe,
                                            channel,
                                            value,
                                        );

                                        if let Err(e) = event_tx.send(BridgeEvent::ToClasp(msg)).await {
                                            debug!("Failed to send sACN data to CLASP: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Timeout is expected, other errors should be logged
                            if !e.to_string().contains("timeout") && !e.to_string().contains("WouldBlock") {
                                debug!("sACN receive error: {}", e);
                            }
                        }
                    }
                }
            }
        }

        *running.lock() = false;
        let _ = event_tx
            .send(BridgeEvent::Disconnected {
                reason: Some("Receiver stopped".to_string()),
            })
            .await;
    }

    /// Run sender mode
    async fn run_sender(
        config: SacnBridgeConfig,
        dmx_data: Arc<Mutex<HashMap<u16, [u8; 512]>>>,
        event_tx: mpsc::Sender<BridgeEvent>,
        mut data_rx: mpsc::Receiver<(u16, u16, u8)>,
        mut shutdown_rx: mpsc::Receiver<()>,
        running: Arc<Mutex<bool>>,
    ) {
        // Create source
        let mut source = match SacnSource::new_v4(&config.source_name) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create sACN source: {}", e);
                let _ = event_tx
                    .send(BridgeEvent::Error(format!("sACN source error: {}", e)))
                    .await;
                return;
            }
        };

        // Register universes
        for &universe in &config.universes {
            if let Err(e) = source.register_universe(universe) {
                warn!("Failed to register universe {}: {}", universe, e);
            }
        }

        *running.lock() = true;
        let _ = event_tx.send(BridgeEvent::Connected).await;
        info!(
            "sACN sender started, source: {}, universes: {:?}",
            config.source_name, config.universes
        );

        // Transmission interval (44Hz is typical for DMX)
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(23));
        let mut dirty_universes: std::collections::HashSet<u16> = std::collections::HashSet::new();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("sACN sender shutting down");
                    // Send termination packets
                    for &universe in &config.universes {
                        let _ = source.terminate_stream(universe, 0);
                    }
                    break;
                }
                // Receive DMX data updates from CLASP
                Some((universe, channel, value)) = data_rx.recv() => {
                    if channel >= 1 && channel <= 512 {
                        let mut data = dmx_data.lock();
                        if let Some(universe_data) = data.get_mut(&universe) {
                            universe_data[(channel - 1) as usize] = value;
                            dirty_universes.insert(universe);
                        }
                    }
                }
                // Transmit on interval
                _ = interval.tick() => {
                    if !dirty_universes.is_empty() {
                        let data = dmx_data.lock();
                        for &universe in &dirty_universes {
                            if let Some(universe_data) = data.get(&universe) {
                                // For multicast, destination is None
                                // For unicast, we send to each destination separately
                                if config.multicast {
                                    if let Err(e) = source.send(
                                        &[universe],
                                        universe_data,
                                        Some(config.priority),
                                        None,
                                        None, // sync address
                                    ) {
                                        debug!("sACN send error for universe {}: {}", universe, e);
                                    }
                                } else {
                                    // Send to each unicast destination
                                    for dest_str in &config.unicast_destinations {
                                        if let Ok(dest) = dest_str.parse::<SocketAddr>() {
                                            if let Err(e) = source.send(
                                                &[universe],
                                                universe_data,
                                                Some(config.priority),
                                                Some(dest),
                                                None,
                                            ) {
                                                debug!("sACN send error to {}: {}", dest, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        dirty_universes.clear();
                    }
                }
            }
        }

        *running.lock() = false;
        let _ = event_tx
            .send(BridgeEvent::Disconnected {
                reason: Some("Sender stopped".to_string()),
            })
            .await;
    }
}

#[async_trait]
impl Bridge for SacnBridge {
    fn config(&self) -> &TraitBridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let (event_tx, event_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        self.shutdown_tx = Some(shutdown_tx);

        let config = self.sacn_config.clone();
        let running = self.running.clone();

        match config.mode {
            SacnMode::Receiver => {
                tokio::spawn(Self::run_receiver(config, event_tx, shutdown_rx, running));
            }
            SacnMode::Sender => {
                let (send_tx, send_rx) = mpsc::channel(1000);
                self.send_tx = Some(send_tx);

                let dmx_data = self.dmx_data.clone();
                tokio::spawn(Self::run_sender(
                    config,
                    dmx_data,
                    event_tx,
                    send_rx,
                    shutdown_rx,
                    running,
                ));
            }
            SacnMode::Bidirectional => {
                // For bidirectional, we need separate shutdown channels
                let (shutdown_tx2, shutdown_rx2) = mpsc::channel(1);
                let (send_tx, send_rx) = mpsc::channel(1000);
                self.send_tx = Some(send_tx);
                // Store the extra shutdown sender
                let _ = self.shutdown_tx.replace(shutdown_tx2);

                let config2 = config.clone();
                let dmx_data = self.dmx_data.clone();
                let event_tx2 = event_tx.clone();
                let running2 = self.running.clone();

                tokio::spawn(Self::run_receiver(
                    config,
                    event_tx,
                    shutdown_rx,
                    running.clone(),
                ));
                tokio::spawn(Self::run_sender(
                    config2,
                    dmx_data,
                    event_tx2,
                    send_rx,
                    shutdown_rx2,
                    running2,
                ));
            }
        }

        info!(
            "sACN bridge started in {:?} mode for universes {:?}",
            self.sacn_config.mode, self.sacn_config.universes
        );
        Ok(event_rx)
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        *self.running.lock() = false;
        info!("sACN bridge stopped");
        Ok(())
    }

    async fn send(&self, msg: Message) -> Result<()> {
        // Handle SET messages to send DMX data
        if let Message::Set(set) = msg {
            if let Some((universe, channel)) =
                Self::parse_address(&self.sacn_config.namespace, &set.address)
            {
                // Convert value to DMX
                let dmx_value = match set.value {
                    Value::Int(v) => (v.clamp(0, 255)) as u8,
                    Value::Float(v) => ((v * 255.0).clamp(0.0, 255.0)) as u8,
                    _ => return Ok(()),
                };

                // Check if this universe is configured
                if !self.sacn_config.universes.contains(&universe) {
                    return Ok(());
                }

                // Update local cache
                {
                    let mut data = self.dmx_data.lock();
                    if let Some(universe_data) = data.get_mut(&universe) {
                        universe_data[(channel - 1) as usize] = dmx_value;
                    }
                }

                // Send to sender task if in sender mode
                if let Some(ref tx) = self.send_tx {
                    let _ = tx.send((universe, channel, dmx_value)).await;
                }

                debug!(
                    "sACN: Set universe {} channel {} = {}",
                    universe, channel, dmx_value
                );
            }
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.sacn_config.namespace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SacnBridgeConfig::default();
        assert_eq!(config.mode, SacnMode::Receiver);
        assert_eq!(config.universes, vec![1]);
        assert_eq!(config.priority, 100);
        assert!(config.multicast);
    }

    #[test]
    fn test_address_parsing() {
        assert_eq!(
            SacnBridge::parse_address("/sacn", "/sacn/1/47"),
            Some((1, 47))
        );
        assert_eq!(
            SacnBridge::parse_address("/sacn", "/sacn/100/512"),
            Some((100, 512))
        );
        assert_eq!(SacnBridge::parse_address("/sacn", "/sacn/1/0"), None); // Channel 0 invalid
        assert_eq!(SacnBridge::parse_address("/sacn", "/sacn/1/513"), None); // Channel > 512
        assert_eq!(SacnBridge::parse_address("/sacn", "/dmx/1/47"), None); // Wrong namespace
    }

    #[test]
    fn test_to_clasp_message() {
        let msg = SacnBridge::to_clasp_message("/sacn", 1, 47, 255);
        if let Message::Set(set) = msg {
            assert_eq!(set.address, "/sacn/1/47");
            assert_eq!(set.value, Value::Int(255));
        } else {
            panic!("Expected SET message");
        }
    }
}
