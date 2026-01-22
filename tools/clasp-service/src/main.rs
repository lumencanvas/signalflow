//! CLASP Bridge Service
//!
//! A JSON-RPC style service that can be spawned by Electron to manage protocol bridges.
//! Communicates via stdin/stdout with JSON messages.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use clasp_bridge::{Bridge, BridgeEvent};
use clasp_core::{Message, SetMessage, Value};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};
use uuid::Uuid;

// Import all bridge types
#[cfg(feature = "osc")]
use clasp_bridge::{OscBridge, OscBridgeConfig};

#[cfg(feature = "midi")]
use clasp_bridge::{MidiBridge, MidiBridgeConfig};

#[cfg(feature = "artnet")]
use clasp_bridge::{ArtNetBridge, ArtNetBridgeConfig};

#[cfg(feature = "dmx")]
use clasp_bridge::{DmxBridge, DmxBridgeConfig, DmxInterfaceType};

#[cfg(feature = "mqtt")]
use clasp_bridge::{MqttBridge, MqttBridgeConfig};

#[cfg(feature = "websocket")]
use clasp_bridge::{WebSocketBridge, WebSocketBridgeConfig, WsMode};

#[cfg(feature = "http")]
use clasp_bridge::{HttpBridge, HttpBridgeConfig, HttpMode};

#[cfg(feature = "socketio")]
use clasp_bridge::{SocketIOBridge, SocketIOBridgeConfig};

#[cfg(feature = "sacn")]
use clasp_bridge::{SacnBridge, SacnBridgeConfig, SacnMode};

/// Request from Electron
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Request {
    #[serde(rename = "create_bridge")]
    CreateBridge {
        id: Option<String>,
        source: String,
        source_addr: String,
        target: String,
        target_addr: String,
        #[serde(default)]
        config: Option<serde_json::Value>,
    },
    #[serde(rename = "delete_bridge")]
    DeleteBridge { id: String },
    #[serde(rename = "list_bridges")]
    ListBridges,
    #[serde(rename = "get_diagnostics")]
    GetDiagnostics { bridge_id: Option<String> },
    #[serde(rename = "health_check")]
    HealthCheck,
    #[serde(rename = "send_signal")]
    SendSignal {
        bridge_id: String,
        address: String,
        value: serde_json::Value,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Response to Electron
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Response {
    #[serde(rename = "ok")]
    Ok { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "signal")]
    Signal {
        bridge_id: String,
        address: String,
        value: serde_json::Value,
    },
    #[serde(rename = "bridge_event")]
    BridgeEvent {
        bridge_id: String,
        event: String,
        data: Option<String>,
    },
    #[serde(rename = "ready")]
    Ready,
}

/// Bridge info for listing
#[derive(Debug, Clone, Serialize)]
struct BridgeInfo {
    id: String,
    source: String,
    source_addr: String,
    target: String,
    target_addr: String,
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
    messages_sent: u64,
    messages_received: u64,
}

/// Detailed diagnostics for a bridge
#[derive(Debug, Clone, Serialize)]
struct BridgeDiagnostics {
    id: String,
    protocol: String,
    status: BridgeStatus,
    config: serde_json::Value,
    metrics: BridgeMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_activity: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    recent_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum BridgeStatus {
    Starting,
    Running,
    Stopped,
    Error,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Default)]
struct BridgeMetrics {
    messages_sent: u64,
    messages_received: u64,
    bytes_sent: u64,
    bytes_received: u64,
    errors: u64,
    reconnects: u64,
}

/// Active bridge handle
struct ActiveBridge {
    info: BridgeInfo,
    bridge: Box<dyn Bridge>,
    started_at: std::time::Instant,
    metrics: Arc<RwLock<BridgeMetrics>>,
    recent_errors: Arc<RwLock<Vec<String>>>,
}

/// Bridge service state
struct BridgeService {
    bridges: RwLock<HashMap<String, ActiveBridge>>,
    signal_tx: mpsc::Sender<Response>,
}

impl BridgeService {
    fn new(signal_tx: mpsc::Sender<Response>) -> Self {
        Self {
            bridges: RwLock::new(HashMap::new()),
            signal_tx,
        }
    }

    async fn create_bridge(
        &self,
        id: Option<String>,
        source: String,
        source_addr: String,
        target: String,
        target_addr: String,
        extra_config: Option<serde_json::Value>,
    ) -> Result<BridgeInfo> {
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

        // Create the appropriate bridge based on source protocol
        let bridge: Box<dyn Bridge> = match source.as_str() {
            #[cfg(feature = "osc")]
            "osc" => {
                let config = OscBridgeConfig {
                    bind_addr: source_addr.clone(),
                    remote_addr: if target == "osc" {
                        Some(target_addr.clone())
                    } else {
                        None
                    },
                    namespace: "/osc".to_string(),
                };
                Box::new(OscBridge::new(config))
            }

            #[cfg(feature = "midi")]
            "midi" => {
                let config = MidiBridgeConfig {
                    input_port: if source_addr == "default" {
                        None
                    } else {
                        Some(source_addr.clone())
                    },
                    output_port: if target == "midi" && target_addr != "default" {
                        Some(target_addr.clone())
                    } else {
                        None
                    },
                    namespace: "/midi".to_string(),
                    device_name: "default".to_string(),
                };
                Box::new(MidiBridge::new(config))
            }

            #[cfg(feature = "artnet")]
            "artnet" => {
                let universes = extra_config
                    .as_ref()
                    .and_then(|c| c.get("universe"))
                    .and_then(|v| v.as_u64())
                    .map(|u| vec![u as u16])
                    .unwrap_or_default();

                let config = ArtNetBridgeConfig {
                    bind_addr: source_addr.clone(),
                    remote_addr: if target == "artnet" {
                        Some(target_addr.clone())
                    } else {
                        None
                    },
                    universes,
                    namespace: "/artnet".to_string(),
                };
                Box::new(ArtNetBridge::new(config))
            }

            #[cfg(feature = "dmx")]
            "dmx" => {
                let universe = extra_config
                    .as_ref()
                    .and_then(|c| c.get("universe"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u16;

                let config = DmxBridgeConfig {
                    port: Some(source_addr.clone()),
                    interface_type: DmxInterfaceType::Virtual,
                    universe,
                    namespace: "/dmx".to_string(),
                    refresh_rate: 44.0,
                };
                Box::new(DmxBridge::new(config))
            }

            #[cfg(feature = "mqtt")]
            "mqtt" => {
                let subscribe_topics = extra_config
                    .as_ref()
                    .and_then(|c| c.get("topics"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| vec!["#".to_string()]);

                // Parse host:port from source_addr
                let (broker_host, broker_port) = if let Some(idx) = source_addr.rfind(':') {
                    let host = source_addr[..idx].to_string();
                    let port = source_addr[idx + 1..].parse().unwrap_or(1883);
                    (host, port)
                } else {
                    (source_addr.clone(), 1883)
                };

                let config = MqttBridgeConfig {
                    broker_host,
                    broker_port,
                    client_id: format!("clasp-bridge-{}", id),
                    username: None,
                    password: None,
                    subscribe_topics,
                    qos: 0,
                    keep_alive_secs: 60,
                    namespace: "/mqtt".to_string(),
                };
                Box::new(MqttBridge::new(config))
            }

            #[cfg(feature = "websocket")]
            "websocket" => {
                let mode_str = extra_config
                    .as_ref()
                    .and_then(|c| c.get("mode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("server");

                let (mode, url) = if mode_str == "client" {
                    // Client mode: source_addr is the WebSocket URL
                    let url =
                        if source_addr.starts_with("ws://") || source_addr.starts_with("wss://") {
                            source_addr.clone()
                        } else {
                            format!("ws://{}", source_addr)
                        };
                    (WsMode::Client, url)
                } else {
                    // Server mode: source_addr is bind address
                    (WsMode::Server, source_addr.clone())
                };

                let config = WebSocketBridgeConfig {
                    mode,
                    url,
                    path: None,
                    format: clasp_bridge::WsMessageFormat::Json,
                    ping_interval_secs: 30,
                    auto_reconnect: true,
                    reconnect_delay_secs: 5,
                    headers: std::collections::HashMap::new(),
                    namespace: "/ws".to_string(),
                };
                Box::new(WebSocketBridge::new(config))
            }

            #[cfg(feature = "http")]
            "http" => {
                let base_path = extra_config
                    .as_ref()
                    .and_then(|c| c.get("base_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("/api")
                    .to_string();

                let cors = extra_config
                    .as_ref()
                    .and_then(|c| c.get("cors"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let config = HttpBridgeConfig {
                    mode: HttpMode::Server,
                    url: source_addr.clone(),
                    endpoints: vec![],
                    cors_enabled: cors,
                    cors_origins: vec![],
                    base_path,
                    timeout_secs: 30,
                    namespace: "/http".to_string(),
                    poll_interval_ms: 0,
                    poll_endpoints: vec![],
                };
                Box::new(HttpBridge::new(config))
            }

            #[cfg(feature = "socketio")]
            "socketio" => {
                let sio_namespace = extra_config
                    .as_ref()
                    .and_then(|c| c.get("sio_namespace"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("/")
                    .to_string();

                let events = extra_config
                    .as_ref()
                    .and_then(|c| c.get("events"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|e| e.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| vec!["message".to_string()]);

                let auth = extra_config.as_ref().and_then(|c| c.get("auth")).cloned();

                let config = SocketIOBridgeConfig {
                    url: source_addr.clone(),
                    sio_namespace,
                    events,
                    auth,
                    reconnect: true,
                    namespace: "/socketio".to_string(),
                };
                Box::new(SocketIOBridge::new(config))
            }

            #[cfg(feature = "sacn")]
            "sacn" => {
                // Parse mode from config
                let mode = extra_config
                    .as_ref()
                    .and_then(|c| c.get("mode"))
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "sender" => SacnMode::Sender,
                        "bidirectional" => SacnMode::Bidirectional,
                        _ => SacnMode::Receiver,
                    })
                    .unwrap_or(SacnMode::Receiver);

                // Parse universes from config or default to [1]
                let universes: Vec<u16> = extra_config
                    .as_ref()
                    .and_then(|c| c.get("universes"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as u16))
                            .collect()
                    })
                    .unwrap_or_else(|| vec![1]);

                let priority = extra_config
                    .as_ref()
                    .and_then(|c| c.get("priority"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u8)
                    .unwrap_or(100);

                let source_name = extra_config
                    .as_ref()
                    .and_then(|c| c.get("source_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("CLASP sACN Bridge")
                    .to_string();

                let multicast = extra_config
                    .as_ref()
                    .and_then(|c| c.get("multicast"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let unicast_destinations: Vec<String> = extra_config
                    .as_ref()
                    .and_then(|c| c.get("unicast_destinations"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let config = SacnBridgeConfig {
                    mode,
                    universes,
                    source_name,
                    priority,
                    bind_address: if source_addr.is_empty() {
                        None
                    } else {
                        Some(source_addr.clone())
                    },
                    multicast,
                    unicast_destinations,
                    namespace: "/sacn".to_string(),
                    preview: false,
                    sync_address: 0,
                };
                Box::new(SacnBridge::new(config))
            }

            _ => {
                return Err(anyhow!("Unsupported source protocol: {}", source));
            }
        };

        // Start the bridge
        let signal_tx = self.signal_tx.clone();
        let bridge_id = id.clone();
        let mut bridge = bridge;

        // Create metrics tracking
        let metrics = Arc::new(RwLock::new(BridgeMetrics::default()));
        let recent_errors = Arc::new(RwLock::new(Vec::new()));
        let metrics_clone = metrics.clone();
        let errors_clone = recent_errors.clone();
        let started_at = std::time::Instant::now();
        let start_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        match bridge.start().await {
            Ok(mut event_rx) => {
                // Spawn task to handle bridge events
                tokio::spawn(async move {
                    while let Some(event) = event_rx.recv().await {
                        match event {
                            BridgeEvent::ToClasp(msg) => {
                                // Track received messages
                                {
                                    let mut m = metrics_clone.write().await;
                                    m.messages_received += 1;
                                }

                                // Extract address and value from the message
                                let (address, value) = match &msg {
                                    Message::Set(set) => {
                                        (set.address.clone(), value_to_json(&set.value))
                                    }
                                    Message::Publish(pub_msg) => {
                                        let val = pub_msg
                                            .value
                                            .as_ref()
                                            .map(value_to_json)
                                            .unwrap_or(serde_json::json!(null));
                                        (pub_msg.address.clone(), val)
                                    }
                                    _ => continue,
                                };

                                let _ = signal_tx
                                    .send(Response::Signal {
                                        bridge_id: bridge_id.clone(),
                                        address,
                                        value,
                                    })
                                    .await;
                            }
                            BridgeEvent::Connected => {
                                let _ = signal_tx
                                    .send(Response::BridgeEvent {
                                        bridge_id: bridge_id.clone(),
                                        event: "connected".to_string(),
                                        data: None,
                                    })
                                    .await;
                            }
                            BridgeEvent::Disconnected { reason } => {
                                // Track reconnects
                                {
                                    let mut m = metrics_clone.write().await;
                                    m.reconnects += 1;
                                }
                                let _ = signal_tx
                                    .send(Response::BridgeEvent {
                                        bridge_id: bridge_id.clone(),
                                        event: "disconnected".to_string(),
                                        data: reason,
                                    })
                                    .await;
                            }
                            BridgeEvent::Error(e) => {
                                // Track errors
                                {
                                    let mut m = metrics_clone.write().await;
                                    m.errors += 1;
                                }
                                {
                                    let mut errs = errors_clone.write().await;
                                    errs.push(e.clone());
                                    // Keep only last 10 errors
                                    if errs.len() > 10 {
                                        errs.remove(0);
                                    }
                                }
                                let _ = signal_tx
                                    .send(Response::BridgeEvent {
                                        bridge_id: bridge_id.clone(),
                                        event: "error".to_string(),
                                        data: Some(e),
                                    })
                                    .await;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                return Err(anyhow!("Failed to start bridge: {}", e));
            }
        }

        let info = BridgeInfo {
            id: id.clone(),
            source,
            source_addr,
            target,
            target_addr,
            active: true,
            started_at: Some(start_timestamp),
            uptime_secs: None, // Computed dynamically
            last_error: None,
            messages_sent: 0,
            messages_received: 0,
        };

        let active_bridge = ActiveBridge {
            info: info.clone(),
            bridge,
            started_at,
            metrics,
            recent_errors,
        };

        self.bridges.write().await.insert(id, active_bridge);

        Ok(info)
    }

    async fn delete_bridge(&self, id: &str) -> Result<()> {
        let mut bridges = self.bridges.write().await;
        if let Some(mut bridge) = bridges.remove(id) {
            bridge.bridge.stop().await?;
            Ok(())
        } else {
            Err(anyhow!("Bridge not found: {}", id))
        }
    }

    async fn list_bridges(&self) -> Vec<BridgeInfo> {
        let bridges = self.bridges.read().await;
        let mut result = Vec::new();

        for b in bridges.values() {
            let metrics = b.metrics.read().await;
            let errors = b.recent_errors.read().await;
            let uptime = b.started_at.elapsed().as_secs();

            result.push(BridgeInfo {
                id: b.info.id.clone(),
                source: b.info.source.clone(),
                source_addr: b.info.source_addr.clone(),
                target: b.info.target.clone(),
                target_addr: b.info.target_addr.clone(),
                active: b.bridge.is_running(),
                started_at: b.info.started_at,
                uptime_secs: Some(uptime),
                last_error: errors.last().cloned(),
                messages_sent: metrics.messages_sent,
                messages_received: metrics.messages_received,
            });
        }

        result
    }

    async fn get_diagnostics(&self, bridge_id: Option<String>) -> Result<serde_json::Value> {
        let bridges = self.bridges.read().await;

        if let Some(id) = bridge_id {
            // Get diagnostics for specific bridge
            if let Some(b) = bridges.get(&id) {
                let metrics = b.metrics.read().await.clone();
                let errors = b.recent_errors.read().await.clone();
                let uptime = b.started_at.elapsed().as_secs();

                let status = if b.bridge.is_running() {
                    BridgeStatus::Running
                } else if !errors.is_empty() {
                    BridgeStatus::Error
                } else {
                    BridgeStatus::Stopped
                };

                let diag = BridgeDiagnostics {
                    id: b.info.id.clone(),
                    protocol: b.info.source.clone(),
                    status,
                    config: serde_json::json!({
                        "source_addr": b.info.source_addr,
                        "target": b.info.target,
                        "target_addr": b.info.target_addr,
                    }),
                    metrics,
                    last_activity: Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    ),
                    recent_errors: errors,
                };

                Ok(serde_json::to_value(diag).unwrap_or(serde_json::json!(null)))
            } else {
                Err(anyhow!("Bridge not found: {}", id))
            }
        } else {
            // Get summary diagnostics for all bridges
            let mut all_diags = Vec::new();

            for b in bridges.values() {
                let metrics = b.metrics.read().await.clone();
                let errors = b.recent_errors.read().await.clone();

                let status = if b.bridge.is_running() {
                    BridgeStatus::Running
                } else if !errors.is_empty() {
                    BridgeStatus::Error
                } else {
                    BridgeStatus::Stopped
                };

                all_diags.push(BridgeDiagnostics {
                    id: b.info.id.clone(),
                    protocol: b.info.source.clone(),
                    status,
                    config: serde_json::json!({
                        "source_addr": b.info.source_addr,
                        "target": b.info.target,
                        "target_addr": b.info.target_addr,
                    }),
                    metrics,
                    last_activity: None,
                    recent_errors: errors,
                });
            }

            Ok(serde_json::to_value(all_diags).unwrap_or(serde_json::json!([])))
        }
    }

    async fn health_check(&self) -> serde_json::Value {
        let bridges = self.bridges.read().await;
        let total = bridges.len();
        let running = bridges.values().filter(|b| b.bridge.is_running()).count();
        let errors: u64 = {
            let mut sum = 0u64;
            for b in bridges.values() {
                sum += b.metrics.read().await.errors;
            }
            sum
        };

        serde_json::json!({
            "status": if running == total && total > 0 { "healthy" } else if running > 0 { "degraded" } else { "idle" },
            "bridges_total": total,
            "bridges_running": running,
            "bridges_stopped": total - running,
            "total_errors": errors,
            "uptime_secs": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        })
    }

    async fn send_signal(
        &self,
        bridge_id: &str,
        address: String,
        value: serde_json::Value,
    ) -> Result<()> {
        let bridges = self.bridges.read().await;
        if let Some(bridge) = bridges.get(bridge_id) {
            let sf_value = json_to_value(&value);
            let msg = Message::Set(SetMessage {
                address,
                value: sf_value,
                revision: None,
                lock: false,
                unlock: false,
            });
            bridge.bridge.send(msg).await?;
            Ok(())
        } else {
            Err(anyhow!("Bridge not found: {}", bridge_id))
        }
    }
}

/// Convert CLASP Value to JSON
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::json!(null),
        Value::Bool(b) => serde_json::json!(b),
        Value::Int(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::String(s) => serde_json::json!(s),
        Value::Bytes(b) => serde_json::json!(base64_encode(b)),
        Value::Array(arr) => {
            serde_json::json!(arr.iter().map(value_to_json).collect::<Vec<_>>())
        }
        Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

/// Convert JSON to CLASP Value
fn json_to_value(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => Value::Array(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect();
            Value::Map(map)
        }
    }
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("CLASP Bridge Service starting...");

    // Channel for sending async responses
    let (tx, mut rx) = mpsc::channel::<Response>(100);
    let service = Arc::new(BridgeService::new(tx));

    // Spawn stdout writer for async events
    let stdout_handle = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(response) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = stdout.write_all(json.as_bytes()).await;
                let _ = stdout.write_all(b"\n").await;
                let _ = stdout.flush().await;
            }
        }
    });

    // Send ready signal
    let mut stdout = tokio::io::stdout();
    let ready = serde_json::to_string(&Response::Ready)?;
    stdout.write_all(ready.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;

    info!("Bridge service ready");

    // Read commands from stdin
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - parent process closed stdin
                info!("EOF received, shutting down");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                debug!("Received: {}", trimmed);

                let response = match serde_json::from_str::<Request>(trimmed) {
                    Ok(request) => {
                        let is_shutdown = matches!(request, Request::Shutdown);
                        let resp = handle_request(&service, request).await;

                        // Send response
                        if let Ok(json) = serde_json::to_string(&resp) {
                            let mut stdout = tokio::io::stdout();
                            let _ = stdout.write_all(json.as_bytes()).await;
                            let _ = stdout.write_all(b"\n").await;
                            let _ = stdout.flush().await;
                        }

                        if is_shutdown {
                            break;
                        }
                        continue;
                    }
                    Err(e) => Response::Error {
                        message: format!("Invalid JSON: {}", e),
                    },
                };

                // Send error response
                if let Ok(json) = serde_json::to_string(&response) {
                    let mut stdout = tokio::io::stdout();
                    let _ = stdout.write_all(json.as_bytes()).await;
                    let _ = stdout.write_all(b"\n").await;
                    let _ = stdout.flush().await;
                }
            }
            Err(e) => {
                error!("Error reading stdin: {}", e);
                break;
            }
        }
    }

    // Cleanup
    stdout_handle.abort();
    info!("Bridge service stopped");

    Ok(())
}

async fn handle_request(service: &Arc<BridgeService>, request: Request) -> Response {
    match request {
        Request::CreateBridge {
            id,
            source,
            source_addr,
            target,
            target_addr,
            config,
        } => match service
            .create_bridge(id, source, source_addr, target, target_addr, config)
            .await
        {
            Ok(info) => Response::Ok {
                data: serde_json::to_value(info).unwrap_or(serde_json::json!(null)),
            },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        },
        Request::DeleteBridge { id } => match service.delete_bridge(&id).await {
            Ok(()) => Response::Ok {
                data: serde_json::json!({"deleted": id}),
            },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        },
        Request::ListBridges => {
            let bridges = service.list_bridges().await;
            Response::Ok {
                data: serde_json::to_value(bridges).unwrap_or(serde_json::json!([])),
            }
        }
        Request::GetDiagnostics { bridge_id } => match service.get_diagnostics(bridge_id).await {
            Ok(data) => Response::Ok { data },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        },
        Request::HealthCheck => {
            let health = service.health_check().await;
            Response::Ok { data: health }
        }
        Request::SendSignal {
            bridge_id,
            address,
            value,
        } => match service.send_signal(&bridge_id, address, value).await {
            Ok(()) => Response::Ok {
                data: serde_json::json!({"sent": true}),
            },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        },
        Request::Ping => Response::Ok {
            data: serde_json::json!({"pong": true}),
        },
        Request::Shutdown => Response::Ok {
            data: serde_json::json!({"shutdown": true}),
        },
    }
}
