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

#[cfg(feature = "rendezvous")]
pub use rendezvous::{DeviceRegistration, RendezvousClient, RendezvousConfig, RendezvousServer};

use std::sync::Arc;
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

/// Discovery source (where the device was discovered from)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    /// mDNS/Bonjour (LAN)
    Mdns,
    /// UDP broadcast (LAN)
    Broadcast,
    /// Rendezvous server (WAN)
    Rendezvous,
    /// Manually added
    Manual,
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
    /// Rendezvous server URL for WAN discovery (e.g., "https://rendezvous.example.com")
    pub rendezvous_url: Option<String>,
    /// Rendezvous refresh interval (how often to re-register, should be < TTL)
    pub rendezvous_refresh_interval: Duration,
    /// Filter tag for rendezvous discovery
    pub rendezvous_tag: Option<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns: true,
            broadcast: true,
            broadcast_port: clasp_core::DEFAULT_DISCOVERY_PORT,
            timeout: Duration::from_secs(5),
            rendezvous_url: None,
            rendezvous_refresh_interval: Duration::from_secs(120), // 2 minutes (< 5 min default TTL)
            rendezvous_tag: None,
        }
    }
}

/// Rendezvous keepalive state
#[cfg(feature = "rendezvous")]
struct RendezvousKeepalive {
    client: rendezvous::RendezvousClient,
    registration: rendezvous::DeviceRegistration,
    device_id: parking_lot::RwLock<Option<String>>,
    refresh_interval: Duration,
}

#[cfg(feature = "rendezvous")]
impl RendezvousKeepalive {
    fn new(
        url: &str,
        registration: rendezvous::DeviceRegistration,
        refresh_interval: Duration,
    ) -> Self {
        Self {
            client: rendezvous::RendezvousClient::new(url),
            registration,
            device_id: parking_lot::RwLock::new(None),
            refresh_interval,
        }
    }

    async fn register(&self) -> Result<()> {
        let response = self
            .client
            .register(self.registration.clone())
            .await
            .map_err(|e| DiscoveryError::Other(format!("Rendezvous registration failed: {}", e)))?;

        *self.device_id.write() = Some(response.id);
        tracing::info!(
            "Registered with rendezvous server (TTL: {}s)",
            response.ttl
        );
        Ok(())
    }

    async fn refresh(&self) -> Result<bool> {
        let device_id: Option<String> = self.device_id.read().clone();
        if let Some(ref id) = device_id {
            let success = self
                .client
                .refresh(id)
                .await
                .map_err(|e| DiscoveryError::Other(format!("Rendezvous refresh failed: {}", e)))?;

            if !success {
                // Device was removed, re-register
                tracing::warn!("Rendezvous registration expired, re-registering");
                *self.device_id.write() = None;
                self.register().await?;
            }
            Ok(true)
        } else {
            // Not registered yet, register now
            self.register().await?;
            Ok(true)
        }
    }

    async fn unregister(&self) -> Result<()> {
        let device_id: Option<String> = self.device_id.write().take();
        if let Some(ref id) = device_id {
            let _ = self.client.unregister(id).await;
            tracing::info!("Unregistered from rendezvous server");
        }
        Ok(())
    }

    /// Start the keepalive loop
    fn start_keepalive(self: Arc<Self>) {
        let keepalive = Arc::clone(&self);
        tokio::spawn(async move {
            // Initial registration
            if let Err(e) = keepalive.register().await {
                tracing::error!("Initial rendezvous registration failed: {}", e);
            }

            // Refresh loop
            let mut interval = tokio::time::interval(keepalive.refresh_interval);
            loop {
                interval.tick().await;
                if let Err(e) = keepalive.refresh().await {
                    tracing::warn!("Rendezvous refresh failed: {}", e);
                }
            }
        });
    }
}

/// Discover Clasp devices
pub struct Discovery {
    config: DiscoveryConfig,
    devices: std::collections::HashMap<String, Device>,
    #[cfg(feature = "rendezvous")]
    rendezvous_keepalive: Option<Arc<RendezvousKeepalive>>,
}

impl Discovery {
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
            devices: std::collections::HashMap::new(),
            #[cfg(feature = "rendezvous")]
            rendezvous_keepalive: None,
        }
    }

    pub fn with_config(config: DiscoveryConfig) -> Self {
        Self {
            config,
            devices: std::collections::HashMap::new(),
            #[cfg(feature = "rendezvous")]
            rendezvous_keepalive: None,
        }
    }

    /// Register this device with the rendezvous server and start keepalive
    #[cfg(feature = "rendezvous")]
    pub fn register_with_rendezvous(&mut self, registration: rendezvous::DeviceRegistration) {
        if let Some(ref url) = self.config.rendezvous_url {
            let keepalive = Arc::new(RendezvousKeepalive::new(
                url,
                registration,
                self.config.rendezvous_refresh_interval,
            ));
            keepalive.clone().start_keepalive();
            self.rendezvous_keepalive = Some(keepalive);
        } else {
            tracing::warn!("Cannot register with rendezvous: no URL configured");
        }
    }

    /// Discover devices from the rendezvous server (WAN discovery)
    #[cfg(feature = "rendezvous")]
    pub async fn discover_wan(&self) -> Result<Vec<Device>> {
        let url = self
            .config
            .rendezvous_url
            .as_ref()
            .ok_or_else(|| DiscoveryError::Other("No rendezvous URL configured".to_string()))?;

        let client = rendezvous::RendezvousClient::new(url);
        let tag = self.config.rendezvous_tag.as_deref();
        let registered_devices = client
            .discover(tag)
            .await
            .map_err(|e| DiscoveryError::Other(format!("Rendezvous discovery failed: {}", e)))?;

        // Convert RegisteredDevice to Device
        let devices: Vec<Device> = registered_devices
            .into_iter()
            .map(|rd| {
                let mut meta = rd.metadata.clone();
                // Add tags to metadata
                if !rd.tags.is_empty() {
                    meta.insert("tags".to_string(), rd.tags.join(","));
                }

                let info = DeviceInfo {
                    version: clasp_core::PROTOCOL_VERSION,
                    features: rd.features,
                    bridge: false,
                    bridge_protocol: None,
                    meta,
                };

                let now = std::time::Instant::now();
                Device {
                    id: rd.id,
                    name: rd.name,
                    info,
                    endpoints: rd.endpoints,
                    discovered_at: now,
                    last_seen: now,
                }
            })
            .collect();

        Ok(devices)
    }

    /// Discover all devices using all available methods (cascade discovery)
    /// Tries: mDNS → broadcast → rendezvous
    /// Returns devices from all successful discovery methods
    pub async fn discover_all(&mut self) -> Result<Vec<Device>> {
        let (tx, mut rx) = mpsc::channel(100);
        let mut all_devices = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Start LAN discovery
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

        // Collect LAN results with timeout
        let timeout = self.config.timeout;
        let deadline = tokio::time::Instant::now() + timeout;
        drop(tx); // Close sender so rx completes when all spawned tasks finish

        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Some(DiscoveryEvent::Found(device)) => {
                            if seen_ids.insert(device.id.clone()) {
                                self.devices.insert(device.id.clone(), device.clone());
                                all_devices.push(device);
                            }
                        }
                        Some(DiscoveryEvent::Error(e)) => {
                            tracing::warn!("Discovery error: {}", e);
                        }
                        Some(DiscoveryEvent::Lost(_)) | None => break,
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    tracing::debug!("LAN discovery timeout");
                    break;
                }
            }
        }

        // Try WAN discovery if configured
        #[cfg(feature = "rendezvous")]
        if self.config.rendezvous_url.is_some() {
            match self.discover_wan().await {
                Ok(wan_devices) => {
                    for device in wan_devices {
                        if seen_ids.insert(device.id.clone()) {
                            self.devices.insert(device.id.clone(), device.clone());
                            all_devices.push(device);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("WAN discovery failed: {}", e);
                }
            }
        }

        Ok(all_devices)
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

        // Start WAN discovery if configured
        #[cfg(feature = "rendezvous")]
        if self.config.rendezvous_url.is_some() {
            let tx_clone = tx.clone();
            let config = self.config.clone();
            tokio::spawn(async move {
                let url = config.rendezvous_url.as_ref().unwrap();
                let client = rendezvous::RendezvousClient::new(url);
                let tag = config.rendezvous_tag.as_deref();

                match client.discover(tag).await {
                    Ok(devices) => {
                        for rd in devices {
                            let mut meta = rd.metadata.clone();
                            if !rd.tags.is_empty() {
                                meta.insert("tags".to_string(), rd.tags.join(","));
                            }

                            let info = DeviceInfo {
                                version: clasp_core::PROTOCOL_VERSION,
                                features: rd.features,
                                bridge: false,
                                bridge_protocol: None,
                                meta,
                            };

                            let now = std::time::Instant::now();
                            let device = Device {
                                id: rd.id,
                                name: rd.name,
                                info,
                                endpoints: rd.endpoints,
                                discovered_at: now,
                                last_seen: now,
                            };
                            let _ = tx_clone.send(DiscoveryEvent::Found(device)).await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Rendezvous discovery error: {}", e);
                        let _ = tx_clone
                            .send(DiscoveryEvent::Error(format!(
                                "Rendezvous discovery failed: {}",
                                e
                            )))
                            .await;
                    }
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
