//! Rendezvous Server for WAN Discovery
//!
//! A simple HTTP REST API that enables CLASP devices to discover each other
//! across the internet when mDNS/broadcast discovery is not available.
//!
//! ## Protocol
//!
//! Per the CLASP Protocol specification (§3.1.3), the rendezvous server provides:
//!
//! - `POST /api/v1/register` - Register a device with its endpoints
//! - `GET /api/v1/discover` - Discover registered devices (optionally filtered by tag)
//! - `DELETE /api/v1/unregister/{id}` - Unregister a device
//!
//! ## Usage
//!
//! ```no_run
//! use clasp_discovery::rendezvous::{RendezvousServer, RendezvousConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let server = RendezvousServer::new(RendezvousConfig::default());
//! server.serve("0.0.0.0:7340").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Client Usage
//!
//! ```no_run
//! use clasp_discovery::rendezvous::{RendezvousClient, DeviceRegistration};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = RendezvousClient::new("https://rendezvous.example.com");
//!
//! // Register this device
//! client.register(DeviceRegistration {
//!     name: "My Device".to_string(),
//!     public_key: None,
//!     features: vec!["param".to_string(), "event".to_string()],
//!     endpoints: [("ws".to_string(), "wss://my-device.local:7330".to_string())].into(),
//!     tags: vec!["studio".to_string()],
//!     ..Default::default()
//! }).await?;
//!
//! // Discover other devices
//! let devices = client.discover(Some("studio")).await?;
//! # Ok(())
//! # }
//! ```

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{debug, info};

use crate::error::Result;

/// Default rendezvous port
pub const DEFAULT_RENDEZVOUS_PORT: u16 = 7340;

/// Device registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegistration {
    /// Human-readable device name
    pub name: String,
    /// Optional public key for secure identification (base64 encoded)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Supported features (param, event, stream, gesture, timeline)
    #[serde(default)]
    pub features: Vec<String>,
    /// Available endpoints (transport -> URL mapping)
    pub endpoints: HashMap<String, String>,
    /// Tags for filtering (e.g., "studio", "live", "dev")
    #[serde(default)]
    pub tags: Vec<String>,
    /// Device metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for DeviceRegistration {
    fn default() -> Self {
        Self {
            name: "CLASP Device".to_string(),
            public_key: None,
            features: vec!["param".to_string(), "event".to_string()],
            endpoints: HashMap::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationResponse {
    /// Assigned device ID (for unregistration)
    pub id: String,
    /// Server timestamp
    pub timestamp: u64,
    /// TTL in seconds (device should re-register before expiry)
    pub ttl: u64,
}

/// Registered device (includes server-side metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredDevice {
    /// Device ID
    pub id: String,
    /// Device name
    pub name: String,
    /// Public key (if provided)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Supported features
    pub features: Vec<String>,
    /// Available endpoints
    pub endpoints: HashMap<String, String>,
    /// Tags
    pub tags: Vec<String>,
    /// Device metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Registration timestamp
    pub registered_at: u64,
    /// Last seen timestamp
    pub last_seen: u64,
}

/// Discovery query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverQuery {
    /// Filter by tag
    pub tag: Option<String>,
    /// Filter by feature
    pub feature: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
}

/// Internal device state
#[derive(Debug, Clone)]
struct DeviceState {
    registration: DeviceRegistration,
    id: String,
    registered_at: Instant,
    last_seen: Instant,
}

impl DeviceState {
    fn to_registered_device(&self) -> RegisteredDevice {
        let now = clasp_core::time::now();
        let age = self.registered_at.elapsed().as_micros() as u64;
        let last_seen_age = self.last_seen.elapsed().as_micros() as u64;

        RegisteredDevice {
            id: self.id.clone(),
            name: self.registration.name.clone(),
            public_key: self.registration.public_key.clone(),
            features: self.registration.features.clone(),
            endpoints: self.registration.endpoints.clone(),
            tags: self.registration.tags.clone(),
            metadata: self.registration.metadata.clone(),
            registered_at: now.saturating_sub(age),
            last_seen: now.saturating_sub(last_seen_age),
        }
    }
}

/// Rendezvous server configuration
#[derive(Debug, Clone)]
pub struct RendezvousConfig {
    /// Time-to-live for registrations (seconds)
    pub ttl: u64,
    /// Maximum devices per registration source
    pub max_devices_per_source: usize,
    /// Maximum total devices
    pub max_total_devices: usize,
    /// Cleanup interval (seconds)
    pub cleanup_interval: u64,
}

impl Default for RendezvousConfig {
    fn default() -> Self {
        Self {
            ttl: 300,                      // 5 minutes
            max_devices_per_source: 10,    // 10 devices per IP
            max_total_devices: 10000,      // 10k total devices
            cleanup_interval: 60,          // Clean up every minute
        }
    }
}

/// Shared server state
struct ServerState {
    config: RendezvousConfig,
    devices: DashMap<String, DeviceState>,
}

impl ServerState {
    fn new(config: RendezvousConfig) -> Self {
        Self {
            config,
            devices: DashMap::new(),
        }
    }

    fn register(&self, registration: DeviceRegistration) -> Result<RegistrationResponse> {
        // Check capacity
        if self.devices.len() >= self.config.max_total_devices {
            // Remove oldest device to make room
            let oldest = self
                .devices
                .iter()
                .min_by_key(|entry| entry.last_seen)
                .map(|entry| entry.key().clone());
            if let Some(id) = oldest {
                self.devices.remove(&id);
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = Instant::now();

        let state = DeviceState {
            registration,
            id: id.clone(),
            registered_at: now,
            last_seen: now,
        };

        self.devices.insert(id.clone(), state);

        Ok(RegistrationResponse {
            id,
            timestamp: clasp_core::time::now(),
            ttl: self.config.ttl,
        })
    }

    fn unregister(&self, id: &str) -> bool {
        self.devices.remove(id).is_some()
    }

    fn discover(&self, query: &DiscoverQuery) -> Vec<RegisteredDevice> {
        let limit = query.limit.unwrap_or(100).min(1000);

        self.devices
            .iter()
            .filter(|entry| {
                // Filter by tag
                if let Some(ref tag) = query.tag {
                    if !entry.registration.tags.contains(tag) {
                        return false;
                    }
                }
                // Filter by feature
                if let Some(ref feature) = query.feature {
                    if !entry.registration.features.contains(feature) {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .map(|entry| entry.to_registered_device())
            .collect()
    }

    fn cleanup_expired(&self) {
        let ttl = Duration::from_secs(self.config.ttl);
        let now = Instant::now();

        self.devices.retain(|_, state| now.duration_since(state.last_seen) < ttl);
    }

    fn refresh(&self, id: &str) -> bool {
        if let Some(mut entry) = self.devices.get_mut(id) {
            entry.last_seen = Instant::now();
            true
        } else {
            false
        }
    }
}

/// Rendezvous HTTP server
pub struct RendezvousServer {
    config: RendezvousConfig,
}

impl RendezvousServer {
    pub fn new(config: RendezvousConfig) -> Self {
        Self { config }
    }

    /// Create the Axum router
    pub fn router(&self) -> Router {
        let state = Arc::new(ServerState::new(self.config.clone()));

        // Start cleanup task
        let cleanup_state = Arc::clone(&state);
        let cleanup_interval = self.config.cleanup_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(cleanup_interval));
            loop {
                interval.tick().await;
                cleanup_state.cleanup_expired();
                debug!(
                    "Cleanup: {} devices registered",
                    cleanup_state.devices.len()
                );
            }
        });

        Router::new()
            .route("/api/v1/register", post(handle_register))
            .route("/api/v1/discover", get(handle_discover))
            .route("/api/v1/unregister/:id", delete(handle_unregister))
            .route("/api/v1/refresh/:id", post(handle_refresh))
            .route("/api/v1/health", get(handle_health))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }

    /// Start the server
    pub async fn serve(&self, addr: &str) -> Result<()> {
        let addr: SocketAddr = addr.parse().map_err(|e| {
            crate::error::DiscoveryError::Other(format!("Invalid address: {}", e))
        })?;

        info!("Rendezvous server listening on {}", addr);

        let router = self.router();
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::error::DiscoveryError::Io(e))?;

        axum::serve(listener, router)
            .await
            .map_err(|e| crate::error::DiscoveryError::Other(format!("Server error: {}", e)))
    }
}

impl Default for RendezvousServer {
    fn default() -> Self {
        Self::new(RendezvousConfig::default())
    }
}

// === HTTP Handlers ===

async fn handle_register(
    State(state): State<Arc<ServerState>>,
    Json(registration): Json<DeviceRegistration>,
) -> std::result::Result<(StatusCode, Json<RegistrationResponse>), (StatusCode, String)> {
    debug!("Registering device: {}", registration.name);

    match state.register(registration) {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn handle_discover(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<DiscoverQuery>,
) -> Json<Vec<RegisteredDevice>> {
    debug!("Discovery query: tag={:?}, feature={:?}", query.tag, query.feature);
    Json(state.discover(&query))
}

async fn handle_unregister(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> StatusCode {
    if state.unregister(&id) {
        debug!("Unregistered device: {}", id);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn handle_refresh(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> StatusCode {
    if state.refresh(&id) {
        debug!("Refreshed device: {}", id);
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn handle_health() -> &'static str {
    "OK"
}

// === Client ===

/// Rendezvous client for device registration and discovery
pub struct RendezvousClient {
    base_url: String,
    client: reqwest::Client,
}

impl RendezvousClient {
    /// Create a new rendezvous client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Register this device with the rendezvous server
    pub async fn register(
        &self,
        registration: DeviceRegistration,
    ) -> std::result::Result<RegistrationResponse, reqwest::Error> {
        let url = format!("{}/api/v1/register", self.base_url);
        self.client
            .post(&url)
            .json(&registration)
            .send()
            .await?
            .json()
            .await
    }

    /// Discover devices from the rendezvous server
    pub async fn discover(
        &self,
        tag: Option<&str>,
    ) -> std::result::Result<Vec<RegisteredDevice>, reqwest::Error> {
        let mut url = format!("{}/api/v1/discover", self.base_url);
        if let Some(t) = tag {
            url = format!("{}?tag={}", url, t);
        }
        self.client.get(&url).send().await?.json().await
    }

    /// Unregister a device
    pub async fn unregister(&self, id: &str) -> std::result::Result<bool, reqwest::Error> {
        let url = format!("{}/api/v1/unregister/{}", self.base_url, id);
        let response = self.client.delete(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// Refresh registration (extend TTL)
    pub async fn refresh(&self, id: &str) -> std::result::Result<bool, reqwest::Error> {
        let url = format!("{}/api/v1/refresh/{}", self.base_url, id);
        let response = self.client.post(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_registration_default() {
        let reg = DeviceRegistration::default();
        assert_eq!(reg.name, "CLASP Device");
        assert!(reg.endpoints.is_empty());
    }

    #[test]
    fn test_server_state_register() {
        let state = ServerState::new(RendezvousConfig::default());
        let registration = DeviceRegistration {
            name: "Test Device".to_string(),
            endpoints: [("ws".to_string(), "ws://localhost:7330".to_string())].into(),
            ..Default::default()
        };

        let response = state.register(registration).unwrap();
        assert!(!response.id.is_empty());
        assert!(response.ttl > 0);
    }

    #[test]
    fn test_server_state_discover() {
        let state = ServerState::new(RendezvousConfig::default());

        // Register two devices with different tags
        state
            .register(DeviceRegistration {
                name: "Studio Device".to_string(),
                tags: vec!["studio".to_string()],
                endpoints: [("ws".to_string(), "ws://studio:7330".to_string())].into(),
                ..Default::default()
            })
            .unwrap();

        state
            .register(DeviceRegistration {
                name: "Live Device".to_string(),
                tags: vec!["live".to_string()],
                endpoints: [("ws".to_string(), "ws://live:7330".to_string())].into(),
                ..Default::default()
            })
            .unwrap();

        // Discover all
        let all = state.discover(&DiscoverQuery {
            tag: None,
            feature: None,
            limit: None,
        });
        assert_eq!(all.len(), 2);

        // Discover by tag
        let studio = state.discover(&DiscoverQuery {
            tag: Some("studio".to_string()),
            feature: None,
            limit: None,
        });
        assert_eq!(studio.len(), 1);
        assert_eq!(studio[0].name, "Studio Device");
    }

    #[test]
    fn test_server_state_unregister() {
        let state = ServerState::new(RendezvousConfig::default());
        let response = state
            .register(DeviceRegistration::default())
            .unwrap();

        assert!(state.unregister(&response.id));
        assert!(!state.unregister(&response.id)); // Already removed
    }

    #[test]
    fn test_server_state_refresh() {
        let state = ServerState::new(RendezvousConfig::default());
        let response = state
            .register(DeviceRegistration::default())
            .unwrap();

        assert!(state.refresh(&response.id));
        assert!(!state.refresh("nonexistent"));
    }
}
