//! HTTP/REST Bridge for CLASP
//!
//! Provides both HTTP server and client capabilities for CLASP.
//! - Server mode: Expose CLASP signals as REST endpoints
//! - Client mode: Bridge HTTP requests to CLASP signals

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get},
    Router,
};
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info};

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

/// HTTP Bridge mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpMode {
    /// HTTP server exposing REST endpoints
    #[default]
    Server,
    /// HTTP client making requests
    Client,
}

/// Endpoint configuration for server mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// HTTP path (e.g., "/api/lights/:id")
    pub path: String,
    /// HTTP method
    #[serde(default)]
    pub method: HttpMethod,
    /// CLASP address to map to/from
    pub clasp_address: String,
    /// Description for documentation
    #[serde(default)]
    pub description: Option<String>,
}

/// HTTP Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBridgeConfig {
    /// Bridge mode (server or client)
    #[serde(default)]
    pub mode: HttpMode,
    /// Bind address for server mode (e.g., "0.0.0.0:3000")
    /// Base URL for client mode (e.g., "https://api.example.com")
    pub url: String,
    /// Configured endpoints
    #[serde(default)]
    pub endpoints: Vec<EndpointConfig>,
    /// Enable CORS (server mode)
    #[serde(default = "default_true")]
    pub cors_enabled: bool,
    /// CORS allowed origins (empty = any)
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Base path prefix for all endpoints
    #[serde(default = "default_base_path")]
    pub base_path: String,
    /// Timeout for client requests in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
    /// CLASP namespace prefix
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_true() -> bool {
    true
}

fn default_base_path() -> String {
    "/api".to_string()
}

fn default_timeout() -> u32 {
    30
}

fn default_namespace() -> String {
    "/http".to_string()
}

impl Default for HttpBridgeConfig {
    fn default() -> Self {
        Self {
            mode: HttpMode::Server,
            url: "0.0.0.0:3000".to_string(),
            endpoints: vec![
                EndpointConfig {
                    path: "/signals".to_string(),
                    method: HttpMethod::GET,
                    clasp_address: "/**".to_string(),
                    description: Some("List all signals".to_string()),
                },
                EndpointConfig {
                    path: "/signals/*path".to_string(),
                    method: HttpMethod::GET,
                    clasp_address: "/{path}".to_string(),
                    description: Some("Get signal value".to_string()),
                },
                EndpointConfig {
                    path: "/signals/*path".to_string(),
                    method: HttpMethod::PUT,
                    clasp_address: "/{path}".to_string(),
                    description: Some("Set signal value".to_string()),
                },
                EndpointConfig {
                    path: "/signals/*path".to_string(),
                    method: HttpMethod::POST,
                    clasp_address: "/{path}".to_string(),
                    description: Some("Publish event".to_string()),
                },
            ],
            cors_enabled: true,
            cors_origins: vec![],
            base_path: "/api".to_string(),
            timeout_secs: 30,
            namespace: "/http".to_string(),
        }
    }
}

/// Shared state for HTTP handlers
#[derive(Clone)]
struct AppState {
    event_tx: mpsc::Sender<BridgeEvent>,
    signals: Arc<parking_lot::RwLock<HashMap<String, Value>>>,
    namespace: String,
}

/// HTTP Bridge implementation
pub struct HttpBridge {
    config: BridgeConfig,
    http_config: HttpBridgeConfig,
    running: Arc<Mutex<bool>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    signals: Arc<parking_lot::RwLock<HashMap<String, Value>>>,
}

impl HttpBridge {
    /// Create a new HTTP bridge
    pub fn new(http_config: HttpBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: "HTTP Bridge".to_string(),
            protocol: "http".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            http_config,
            running: Arc::new(Mutex::new(false)),
            shutdown_tx: None,
            signals: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Convert JSON to CLASP Value
    fn json_to_value(json: serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Self::json_to_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let map: HashMap<String, Value> = obj
                    .into_iter()
                    .map(|(k, v)| (k, Self::json_to_value(v)))
                    .collect();
                Value::Map(map)
            }
        }
    }

    /// Convert CLASP Value to JSON
    fn value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Bytes(b) => serde_json::Value::Array(
                b.iter()
                    .map(|&x| serde_json::Value::Number(x.into()))
                    .collect(),
            ),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::value_to_json).collect())
            }
            Value::Map(m) => serde_json::Value::Object(
                m.iter()
                    .map(|(k, v)| (k.clone(), Self::value_to_json(v)))
                    .collect(),
            ),
        }
    }

    /// Build the router for server mode
    fn build_router(state: AppState, base_path: &str) -> Router {
        Router::new()
            .route(&format!("{}/signals", base_path), get(list_signals))
            .route(
                &format!("{}/*path", base_path),
                get(get_signal)
                    .put(set_signal)
                    .post(publish_event)
                    .delete(delete_signal),
            )
            .route(&format!("{}/health", base_path), get(health_check))
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }

    /// Update local signal cache (called when receiving messages from CLASP)
    pub fn update_signal(&self, address: &str, value: Value) {
        self.signals.write().insert(address.to_string(), value);
    }
}

// HTTP Handlers

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "protocol": "CLASP",
        "version": "0.1.0"
    }))
}

async fn list_signals(State(state): State<AppState>) -> impl IntoResponse {
    let signals = state.signals.read();
    let list: Vec<serde_json::Value> = signals
        .iter()
        .map(|(addr, val)| {
            serde_json::json!({
                "address": addr,
                "value": HttpBridge::value_to_json(val)
            })
        })
        .collect();

    Json(serde_json::json!({
        "signals": list,
        "count": list.len()
    }))
}

async fn get_signal(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let address = format!("/{}", path);
    let signals = state.signals.read();

    if let Some(value) = signals.get(&address) {
        Json(serde_json::json!({
            "address": address,
            "value": HttpBridge::value_to_json(value)
        }))
        .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Signal not found",
                "address": address
            })),
        )
            .into_response()
    }
}

async fn set_signal(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let address = format!("{}/{}", state.namespace, path);

    let value = if let Some(v) = body.get("value") {
        HttpBridge::json_to_value(v.clone())
    } else {
        HttpBridge::json_to_value(body)
    };

    // Store in local state
    state.signals.write().insert(address.clone(), value.clone());

    // Send CLASP message
    let msg = Message::Set(SetMessage {
        address: address.clone(),
        value: value.clone(),
        revision: None,
        lock: false,
        unlock: false,
    });

    if let Err(e) = state.event_tx.send(BridgeEvent::ToSignalFlow(msg)).await {
        error!("Failed to send set event: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Internal error" })),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "address": address,
        "value": HttpBridge::value_to_json(&value),
        "status": "set"
    }))
    .into_response()
}

async fn publish_event(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let address = format!("{}/{}", state.namespace, path);

    let value = if let Some(v) = body.get("value") {
        HttpBridge::json_to_value(v.clone())
    } else {
        HttpBridge::json_to_value(body)
    };

    // Send CLASP publish
    let msg = Message::Publish(PublishMessage {
        address: address.clone(),
        signal: Some(SignalType::Event),
        value: Some(value.clone()),
        payload: None,
        samples: None,
        rate: None,
        id: None,
        phase: None,
        timestamp: None,
    });

    if let Err(e) = state.event_tx.send(BridgeEvent::ToSignalFlow(msg)).await {
        error!("Failed to send publish event: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Internal error" })),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "address": address,
        "value": HttpBridge::value_to_json(&value),
        "status": "published"
    }))
    .into_response()
}

async fn delete_signal(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let address = format!("{}/{}", state.namespace, path);

    // Remove from state
    let removed = state.signals.write().remove(&address);

    if removed.is_some() {
        // Send null value
        let msg = Message::Set(SetMessage {
            address: address.clone(),
            value: Value::Null,
            revision: None,
            lock: false,
            unlock: false,
        });

        let _ = state.event_tx.send(BridgeEvent::ToSignalFlow(msg)).await;

        Json(serde_json::json!({
            "address": address,
            "status": "deleted"
        }))
        .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Signal not found",
                "address": address
            })),
        )
            .into_response()
    }
}

#[async_trait]
impl Bridge for HttpBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let (tx, rx) = mpsc::channel(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        match self.http_config.mode {
            HttpMode::Server => {
                let addr: SocketAddr = self
                    .http_config
                    .url
                    .parse()
                    .map_err(|e| BridgeError::Other(format!("Invalid address: {}", e)))?;

                let app_state = AppState {
                    event_tx: tx.clone(),
                    signals: self.signals.clone(),
                    namespace: self.http_config.namespace.clone(),
                };

                let mut router = Self::build_router(app_state, &self.http_config.base_path);

                // Add CORS if enabled
                if self.http_config.cors_enabled {
                    let cors = CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any);
                    router = router.layer(cors);
                }

                let running = self.running.clone();
                let tx_clone = tx.clone();

                tokio::spawn(async move {
                    let listener = match tokio::net::TcpListener::bind(addr).await {
                        Ok(l) => l,
                        Err(e) => {
                            error!("Failed to bind HTTP server: {}", e);
                            let _ = tx_clone
                                .send(BridgeEvent::Error(format!("Bind failed: {}", e)))
                                .await;
                            return;
                        }
                    };

                    info!("HTTP server listening on {}", addr);
                    *running.lock() = true;
                    let _ = tx_clone.send(BridgeEvent::Connected).await;

                    axum::serve(listener, router)
                        .with_graceful_shutdown(async move {
                            let _ = shutdown_rx.recv().await;
                        })
                        .await
                        .ok();

                    *running.lock() = false;
                    let _ = tx_clone
                        .send(BridgeEvent::Disconnected {
                            reason: Some("Server stopped".to_string()),
                        })
                        .await;
                    info!("HTTP server stopped");
                });

                *self.running.lock() = true;
                info!(
                    "HTTP bridge started in server mode on {}",
                    self.http_config.url
                );
            }
            HttpMode::Client => {
                // Client mode - ready to make requests
                *self.running.lock() = true;
                let _ = tx.send(BridgeEvent::Connected).await;
                info!(
                    "HTTP bridge started in client mode for {}",
                    self.http_config.url
                );
            }
        }

        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        info!("HTTP bridge stopped");
        Ok(())
    }

    async fn send(&self, msg: Message) -> Result<()> {
        if !*self.running.lock() {
            return Err(BridgeError::Other("Not connected".to_string()));
        }

        match self.http_config.mode {
            HttpMode::Server => {
                // Server mode - update local cache for GET requests
                match &msg {
                    Message::Set(set) => {
                        self.signals
                            .write()
                            .insert(set.address.clone(), set.value.clone());
                    }
                    Message::Publish(pub_msg) => {
                        if let Some(value) = &pub_msg.value {
                            self.signals
                                .write()
                                .insert(pub_msg.address.clone(), value.clone());
                        }
                    }
                    _ => {}
                }
                debug!("HTTP server cached CLASP message");
                Ok(())
            }
            HttpMode::Client => {
                // Client mode - make HTTP request
                let (address, value, method) = match &msg {
                    Message::Set(set) => (&set.address, &set.value, HttpMethod::PUT),
                    Message::Publish(pub_msg) => {
                        if let Some(val) = &pub_msg.value {
                            (&pub_msg.address, val, HttpMethod::POST)
                        } else {
                            return Ok(());
                        }
                    }
                    _ => return Ok(()),
                };

                let url = format!("{}{}", self.http_config.url, address);
                let body = Self::value_to_json(value);

                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(
                        self.http_config.timeout_secs as u64,
                    ))
                    .build()
                    .map_err(|e| BridgeError::Other(format!("HTTP client error: {}", e)))?;

                let request = match method {
                    HttpMethod::GET => client.get(&url),
                    HttpMethod::POST => client.post(&url).json(&body),
                    HttpMethod::PUT => client.put(&url).json(&body),
                    HttpMethod::DELETE => client.delete(&url),
                    HttpMethod::PATCH => client.patch(&url).json(&body),
                };

                let response = request
                    .send()
                    .await
                    .map_err(|e| BridgeError::Other(format!("HTTP request failed: {}", e)))?;

                debug!("HTTP {} {} -> {}", method, url, response.status());
                Ok(())
            }
        }
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.http_config.namespace
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::PATCH => write!(f, "PATCH"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HttpBridgeConfig::default();
        assert!(config.cors_enabled);
        assert!(!config.endpoints.is_empty());
    }

    #[test]
    fn test_value_conversion() {
        let json = serde_json::json!({
            "intensity": 0.75,
            "enabled": true,
            "name": "main light"
        });

        let value = HttpBridge::json_to_value(json.clone());
        let back = HttpBridge::value_to_json(&value);

        assert_eq!(json, back);
    }
}
