//! Clasp Discovery
//!
//! Provides device discovery mechanisms:
//! - mDNS/Bonjour for LAN auto-discovery
//! - UDP broadcast fallback
//! - Rendezvous server for WAN discovery
//! - Manual registration

pub mod device;
pub mod error;

#[cfg(feature = "mdns")]
pub mod mdns;

#[cfg(feature = "broadcast")]
pub mod broadcast;

#[cfg(feature = "rendezvous")]
pub mod rendezvous;

pub use device::{Device, DeviceInfo};
pub use error::{DiscoveryError, Result};

use std::time::Duration;
use tokio::sync::mpsc;

/// Discovery event
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// Device discovered
    Found(Device),
    /// Device removed/lost
    Lost(String), // Device ID
    /// Error during discovery
    Error(String),
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable mDNS discovery
    pub mdns: bool,
    /// Enable UDP broadcast discovery
    pub broadcast: bool,
    /// Broadcast port
    pub broadcast_port: u16,
    /// Discovery timeout
    pub timeout: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns: true,
            broadcast: true,
            broadcast_port: clasp_core::DEFAULT_DISCOVERY_PORT,
            timeout: Duration::from_secs(5),
        }
    }
}

/// Discover Clasp devices
pub struct Discovery {
    config: DiscoveryConfig,
    devices: std::collections::HashMap<String, Device>,
}

impl Discovery {
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
            devices: std::collections::HashMap::new(),
        }
    }

    pub fn with_config(config: DiscoveryConfig) -> Self {
        Self {
            config,
            devices: std::collections::HashMap::new(),
        }
    }

    /// Start discovery and return a receiver for events
    pub async fn start(&mut self) -> Result<mpsc::Receiver<DiscoveryEvent>> {
        let (tx, rx) = mpsc::channel(100);

        #[cfg(feature = "mdns")]
        if self.config.mdns {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = mdns::discover(tx_clone).await {
                    tracing::warn!("mDNS discovery error: {}", e);
                }
            });
        }

        #[cfg(feature = "broadcast")]
        if self.config.broadcast {
            let tx_clone = tx.clone();
            let port = self.config.broadcast_port;
            tokio::spawn(async move {
                if let Err(e) = broadcast::discover(port, tx_clone).await {
                    tracing::warn!("Broadcast discovery error: {}", e);
                }
            });
        }

        Ok(rx)
    }

    /// Get currently known devices
    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }

    /// Get a device by ID
    pub fn get(&self, id: &str) -> Option<&Device> {
        self.devices.get(id)
    }

    /// Manually add a device
    pub fn add(&mut self, device: Device) {
        self.devices.insert(device.id.clone(), device);
    }

    /// Remove a device
    pub fn remove(&mut self, id: &str) -> Option<Device> {
        self.devices.remove(id)
    }
}

impl Default for Discovery {
    fn default() -> Self {
        Self::new()
    }
}
