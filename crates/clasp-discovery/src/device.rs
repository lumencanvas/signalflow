//! Device representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

/// A discovered SignalFlow device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Device info
    pub info: DeviceInfo,
    /// Endpoints (transport -> address)
    pub endpoints: HashMap<String, String>,
    /// When the device was discovered
    #[serde(skip, default = "std::time::Instant::now")]
    pub discovered_at: std::time::Instant,
    /// Last seen time
    #[serde(skip, default = "std::time::Instant::now")]
    pub last_seen: std::time::Instant,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Protocol version
    pub version: u8,
    /// Supported features
    pub features: Vec<String>,
    /// Is this a bridge device?
    pub bridge: bool,
    /// Bridge protocol (if bridge)
    pub bridge_protocol: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub meta: HashMap<String, String>,
}

impl Device {
    /// Create a new device
    pub fn new(id: String, name: String) -> Self {
        let now = std::time::Instant::now();
        Self {
            id,
            name,
            info: DeviceInfo::default(),
            endpoints: HashMap::new(),
            discovered_at: now,
            last_seen: now,
        }
    }

    /// Add a WebSocket endpoint
    pub fn with_ws_endpoint(mut self, url: &str) -> Self {
        self.endpoints.insert("ws".to_string(), url.to_string());
        self
    }

    /// Add a UDP endpoint
    pub fn with_udp_endpoint(mut self, addr: SocketAddr) -> Self {
        self.endpoints.insert("udp".to_string(), addr.to_string());
        self
    }

    /// Get the WebSocket URL
    pub fn ws_url(&self) -> Option<&str> {
        self.endpoints.get("ws").map(|s| s.as_str())
    }

    /// Get the UDP address
    pub fn udp_addr(&self) -> Option<SocketAddr> {
        self.endpoints.get("udp").and_then(|s| s.parse().ok())
    }

    /// Update last seen time
    pub fn touch(&mut self) {
        self.last_seen = std::time::Instant::now();
    }

    /// Check if device is stale
    pub fn is_stale(&self, timeout: std::time::Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            version: clasp_core::PROTOCOL_VERSION,
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
            ],
            bridge: false,
            bridge_protocol: None,
            meta: HashMap::new(),
        }
    }
}

impl DeviceInfo {
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    pub fn as_bridge(mut self, protocol: &str) -> Self {
        self.bridge = true;
        self.bridge_protocol = Some(protocol.to_string());
        self
    }
}
