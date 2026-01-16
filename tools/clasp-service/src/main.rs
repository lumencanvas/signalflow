//! SignalFlow Bridge Service
//!
//! A JSON-RPC style service that can be spawned by Electron to manage protocol bridges.
//! Communicates via stdin/stdout with JSON messages.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use clasp_bridge::{Bridge, BridgeEvent, OscBridge, OscBridgeConfig};
use clasp_core::{Message, SetMessage, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};
use uuid::Uuid;

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
    },
    #[serde(rename = "delete_bridge")]
    DeleteBridge { id: String },
    #[serde(rename = "list_bridges")]
    ListBridges,
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
}

/// Active bridge handle
struct ActiveBridge {
    info: BridgeInfo,
    bridge: Box<dyn Bridge>,
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
    ) -> Result<BridgeInfo> {
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

        // Create the appropriate bridge based on source protocol
        let mut bridge: Box<dyn Bridge> = match source.as_str() {
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
            // Other protocols would be added here
            _ => {
                return Err(anyhow!("Unsupported source protocol: {}", source));
            }
        };

        // Start the bridge
        let signal_tx = self.signal_tx.clone();
        let bridge_id = id.clone();

        match bridge.start().await {
            Ok(mut event_rx) => {
                // Spawn task to handle bridge events
                tokio::spawn(async move {
                    while let Some(event) = event_rx.recv().await {
                        match event {
                            BridgeEvent::ToSignalFlow(msg) => {
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
                                let _ = signal_tx
                                    .send(Response::BridgeEvent {
                                        bridge_id: bridge_id.clone(),
                                        event: "disconnected".to_string(),
                                        data: reason,
                                    })
                                    .await;
                            }
                            BridgeEvent::Error(e) => {
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
        };

        let active_bridge = ActiveBridge {
            info: info.clone(),
            bridge,
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
        self.bridges
            .read()
            .await
            .values()
            .map(|b| BridgeInfo {
                id: b.info.id.clone(),
                source: b.info.source.clone(),
                source_addr: b.info.source_addr.clone(),
                target: b.info.target.clone(),
                target_addr: b.info.target_addr.clone(),
                active: b.bridge.is_running(),
            })
            .collect()
    }

    async fn send_signal(&self, bridge_id: &str, address: String, value: serde_json::Value) -> Result<()> {
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

/// Convert SignalFlow Value to JSON
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

/// Convert JSON to SignalFlow Value
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
        serde_json::Value::Array(arr) => {
            Value::Array(arr.iter().map(json_to_value).collect())
        }
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

    info!("SignalFlow Bridge Service starting...");

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
        } => match service
            .create_bridge(id, source, source_addr, target, target_addr)
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
