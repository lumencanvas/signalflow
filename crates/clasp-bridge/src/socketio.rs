//! Socket.IO Bridge for CLASP
//!
//! Provides Socket.IO client connectivity for CLASP.
//! Supports Socket.IO v4 protocol via rust_socketio.

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};
use async_trait::async_trait;
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use futures::FutureExt;
use parking_lot::Mutex;
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Socket.IO Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketIOBridgeConfig {
    /// Server URL (e.g., "http://localhost:3000")
    pub url: String,
    /// Namespace (default: "/")
    #[serde(default = "default_sio_namespace")]
    pub sio_namespace: String,
    /// Events to listen for
    #[serde(default)]
    pub events: Vec<String>,
    /// Authentication payload (JSON)
    #[serde(default)]
    pub auth: Option<serde_json::Value>,
    /// Auto-reconnect on disconnect
    #[serde(default = "default_true")]
    pub reconnect: bool,
    /// CLASP namespace prefix
    #[serde(default = "default_address_prefix")]
    pub namespace: String,
}

fn default_sio_namespace() -> String {
    "/".to_string()
}

fn default_true() -> bool {
    true
}

fn default_address_prefix() -> String {
    "/socketio".to_string()
}

impl Default for SocketIOBridgeConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:3000".to_string(),
            sio_namespace: "/".to_string(),
            events: vec!["message".to_string()],
            auth: None,
            reconnect: true,
            namespace: "/socketio".to_string(),
        }
    }
}

/// Socket.IO Bridge implementation
pub struct SocketIOBridge {
    config: BridgeConfig,
    sio_config: SocketIOBridgeConfig,
    client: Option<Client>,
    running: Arc<Mutex<bool>>,
}

impl SocketIOBridge {
    /// Create a new Socket.IO bridge
    pub fn new(sio_config: SocketIOBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: "Socket.IO Bridge".to_string(),
            protocol: "socketio".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            sio_config,
            client: None,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Convert Socket.IO payload to CLASP Value
    fn payload_to_value(payload: Payload) -> Value {
        match payload {
            Payload::Text(values) => {
                if values.len() == 1 {
                    Self::json_to_value(values.into_iter().next().unwrap())
                } else {
                    Value::Array(values.into_iter().map(Self::json_to_value).collect())
                }
            }
            Payload::Binary(data) => Value::Bytes(data.to_vec()),
            _ => Value::Null,
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
}

#[async_trait]
impl Bridge for SocketIOBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let url = format!("{}{}", self.sio_config.url, self.sio_config.sio_namespace);
        let namespace = self.sio_config.namespace.clone();
        let events = self.sio_config.events.clone();

        let (tx, rx) = mpsc::channel(100);
        let running = self.running.clone();

        // Build client
        let mut builder = ClientBuilder::new(&url);

        // Add auth if present
        if let Some(auth) = &self.sio_config.auth {
            builder = builder.auth(auth.clone());
        }

        // Add reconnect
        if self.sio_config.reconnect {
            builder = builder.reconnect(true);
        }

        // Add connection handler
        let running_conn = running.clone();
        let tx_conn = tx.clone();
        builder = builder.on("connect", move |_, _| {
            let running = running_conn.clone();
            let tx = tx_conn.clone();
            async move {
                info!("Socket.IO connected");
                *running.lock() = true;
                let _ = tx.send(BridgeEvent::Connected).await;
            }
            .boxed()
        });

        // Add disconnect handler
        let running_disc = running.clone();
        let tx_disc = tx.clone();
        builder = builder.on("disconnect", move |_, _| {
            let running = running_disc.clone();
            let tx = tx_disc.clone();
            async move {
                warn!("Socket.IO disconnected");
                *running.lock() = false;
                let _ = tx
                    .send(BridgeEvent::Disconnected {
                        reason: Some("Server disconnect".to_string()),
                    })
                    .await;
            }
            .boxed()
        });

        // Add event handlers for each configured event
        for event in events {
            let prefix = namespace.clone();
            let event_name = event.clone();
            let tx_event = tx.clone();

            builder = builder.on(event.as_str(), move |payload, _| {
                let prefix = prefix.clone();
                let event_name = event_name.clone();
                let tx = tx_event.clone();
                async move {
                    let value = SocketIOBridge::payload_to_value(payload);
                    let address = format!("{}/{}", prefix, event_name);

                    let msg = Message::Set(SetMessage {
                        address,
                        value,
                        revision: None,
                        lock: false,
                        unlock: false,
                    });

                    debug!("Socket.IO received event: {}", event_name);
                    let _ = tx.send(BridgeEvent::ToClasp(msg)).await;
                }
                .boxed()
            });
        }

        // Connect
        let client = builder.connect().await.map_err(|e| {
            BridgeError::ConnectionFailed(format!("Socket.IO connect failed: {:?}", e))
        })?;

        self.client = Some(client);
        *self.running.lock() = true;

        info!(
            "Socket.IO bridge started, connecting to {}",
            self.sio_config.url
        );
        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;
        if let Some(client) = self.client.take() {
            let _ = client.disconnect().await;
        }
        info!("Socket.IO bridge stopped");
        Ok(())
    }

    async fn send(&self, msg: Message) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| BridgeError::Other("Not connected".to_string()))?;

        let (address, value) = match &msg {
            Message::Set(set) => (&set.address, &set.value),
            Message::Publish(pub_msg) => {
                if let Some(val) = &pub_msg.value {
                    (&pub_msg.address, val)
                } else {
                    return Ok(());
                }
            }
            _ => return Ok(()),
        };

        // Extract event name from address (last segment)
        let event = address.rsplit('/').next().unwrap_or("message");

        let json = Self::value_to_json(value);

        client
            .emit(event, json)
            .await
            .map_err(|e| BridgeError::Other(format!("Socket.IO emit failed: {:?}", e)))?;

        debug!("Socket.IO emitted: {}", event);
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.sio_config.namespace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SocketIOBridgeConfig::default();
        assert_eq!(config.sio_namespace, "/");
        assert_eq!(config.namespace, "/socketio");
    }

    #[test]
    fn test_value_conversion() {
        let json = serde_json::json!({
            "x": 1.5,
            "y": 2.5,
            "name": "test"
        });

        let value = SocketIOBridge::json_to_value(json.clone());
        let back = SocketIOBridge::value_to_json(&value);

        assert_eq!(json, back);
    }
}
