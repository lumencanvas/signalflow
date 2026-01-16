//! WebSocket Bridge for CLASP
//!
//! Provides bidirectional WebSocket connectivity for CLASP.
//! Supports both client and server modes.

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};
use async_trait::async_trait;
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::protocol::Message as WsMessage,
    MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};

/// WebSocket message format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageFormat {
    /// JSON text messages
    #[default]
    Json,
    /// MessagePack binary messages
    MsgPack,
    /// Raw binary/text passthrough
    Raw,
}

/// WebSocket bridge mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WsMode {
    /// Connect to a WebSocket server
    #[default]
    Client,
    /// Act as a WebSocket server
    Server,
}

/// WebSocket Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketBridgeConfig {
    /// Mode: client or server
    #[serde(default)]
    pub mode: WsMode,
    /// URL for client mode (ws://...) or bind address for server mode (0.0.0.0:8080)
    pub url: String,
    /// Path for server mode (e.g., "/ws")
    #[serde(default)]
    pub path: Option<String>,
    /// Message format
    #[serde(default)]
    pub format: WsMessageFormat,
    /// Ping interval in seconds (0 to disable)
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u32,
    /// Auto-reconnect on disconnect (client mode only)
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    /// Reconnect delay in seconds
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u32,
    /// Custom headers for client mode
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// CLASP namespace prefix for incoming messages
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_true() -> bool {
    true
}

fn default_ping_interval() -> u32 {
    30
}

fn default_reconnect_delay() -> u32 {
    5
}

fn default_namespace() -> String {
    "/ws".to_string()
}

impl Default for WebSocketBridgeConfig {
    fn default() -> Self {
        Self {
            mode: WsMode::Client,
            url: "ws://localhost:8080".to_string(),
            path: None,
            format: WsMessageFormat::Json,
            ping_interval_secs: 30,
            auto_reconnect: true,
            reconnect_delay_secs: 5,
            headers: HashMap::new(),
            namespace: "/ws".to_string(),
        }
    }
}

/// WebSocket client connection
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket Bridge implementation
pub struct WebSocketBridge {
    config: BridgeConfig,
    ws_config: WebSocketBridgeConfig,
    running: Arc<Mutex<bool>>,
    send_tx: Option<mpsc::Sender<WsMessage>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WebSocketBridge {
    /// Create a new WebSocket bridge
    pub fn new(ws_config: WebSocketBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: "WebSocket Bridge".to_string(),
            protocol: "websocket".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            ws_config,
            running: Arc::new(Mutex::new(false)),
            send_tx: None,
            shutdown_tx: None,
        }
    }

    /// Parse incoming WebSocket message to CLASP
    fn parse_message(msg: &WsMessage, format: WsMessageFormat, prefix: &str) -> Option<Message> {
        match msg {
            WsMessage::Text(text) => match format {
                WsMessageFormat::Json | WsMessageFormat::Raw => {
                    // Try to parse as JSON
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        let address = json
                            .get("address")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}/message", prefix));

                        let value = json
                            .get("value")
                            .map(|v| Self::json_to_value(v.clone()))
                            .or_else(|| {
                                json.get("data").map(|v| Self::json_to_value(v.clone()))
                            })
                            .unwrap_or_else(|| Self::json_to_value(json));

                        Some(Message::Set(SetMessage {
                            address,
                            value,
                            revision: None,
                            lock: false,
                            unlock: false,
                        }))
                    } else {
                        // Plain text message
                        Some(Message::Set(SetMessage {
                            address: format!("{}/text", prefix),
                            value: Value::String(text.clone()),
                            revision: None,
                            lock: false,
                            unlock: false,
                        }))
                    }
                }
                WsMessageFormat::MsgPack => None,
            },
            WsMessage::Binary(data) => match format {
                WsMessageFormat::MsgPack => {
                    // Try to decode as CLASP message
                    if let Ok((msg, _)) = clasp_core::codec::decode(data) {
                        Some(msg)
                    } else {
                        // Return as bytes
                        Some(Message::Set(SetMessage {
                            address: format!("{}/binary", prefix),
                            value: Value::Bytes(data.clone()),
                            revision: None,
                            lock: false,
                            unlock: false,
                        }))
                    }
                }
                WsMessageFormat::Raw | WsMessageFormat::Json => {
                    Some(Message::Set(SetMessage {
                        address: format!("{}/binary", prefix),
                        value: Value::Bytes(data.clone()),
                        revision: None,
                        lock: false,
                        unlock: false,
                    }))
                }
            },
            _ => None,
        }
    }

    /// Convert JSON value to CLASP Value
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

    /// Convert CLASP message to WebSocket message
    fn message_to_ws(msg: &Message, format: WsMessageFormat) -> Option<WsMessage> {
        let (address, value) = match msg {
            Message::Set(set) => (Some(&set.address), Some(&set.value)),
            Message::Publish(pub_msg) => (Some(&pub_msg.address), pub_msg.value.as_ref()),
            _ => return None,
        };

        match format {
            WsMessageFormat::Json => {
                let json = serde_json::json!({
                    "address": address,
                    "value": value,
                });
                Some(WsMessage::Text(json.to_string()))
            }
            WsMessageFormat::MsgPack => {
                if let Ok(encoded) = clasp_core::codec::encode(msg) {
                    Some(WsMessage::Binary(encoded.to_vec()))
                } else {
                    None
                }
            }
            WsMessageFormat::Raw => {
                if let Some(val) = value {
                    match val {
                        Value::String(s) => Some(WsMessage::Text(s.clone())),
                        Value::Bytes(b) => Some(WsMessage::Binary(b.clone())),
                        _ => {
                            let json = serde_json::to_string(val).ok()?;
                            Some(WsMessage::Text(json))
                        }
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Run client mode
    async fn run_client(
        url: String,
        format: WsMessageFormat,
        namespace: String,
        auto_reconnect: bool,
        reconnect_delay: u32,
        event_tx: mpsc::Sender<BridgeEvent>,
        mut send_rx: mpsc::Receiver<WsMessage>,
        mut shutdown_rx: mpsc::Receiver<()>,
        running: Arc<Mutex<bool>>,
    ) {
        loop {
            info!("WebSocket connecting to {}", url);

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    info!("WebSocket connected");
                    *running.lock() = true;
                    let _ = event_tx.send(BridgeEvent::Connected).await;

                    let (mut write, mut read) = ws_stream.split();

                    loop {
                        tokio::select! {
                            // Handle incoming messages
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(ws_msg)) => {
                                        if let Some(clasp_msg) = Self::parse_message(&ws_msg, format, &namespace) {
                                            let _ = event_tx.send(BridgeEvent::ToClasp(clasp_msg)).await;
                                        }
                                    }
                                    Some(Err(e)) => {
                                        error!("WebSocket error: {}", e);
                                        break;
                                    }
                                    None => {
                                        warn!("WebSocket connection closed");
                                        break;
                                    }
                                }
                            }
                            // Handle outgoing messages
                            msg = send_rx.recv() => {
                                if let Some(ws_msg) = msg {
                                    if let Err(e) = write.send(ws_msg).await {
                                        error!("WebSocket send error: {}", e);
                                        break;
                                    }
                                }
                            }
                            // Handle shutdown
                            _ = shutdown_rx.recv() => {
                                info!("WebSocket shutting down");
                                let _ = write.close().await;
                                *running.lock() = false;
                                return;
                            }
                        }
                    }

                    *running.lock() = false;
                    let _ = event_tx
                        .send(BridgeEvent::Disconnected {
                            reason: Some("Connection closed".to_string()),
                        })
                        .await;
                }
                Err(e) => {
                    error!("WebSocket connection failed: {}", e);
                    let _ = event_tx
                        .send(BridgeEvent::Error(format!("Connection failed: {}", e)))
                        .await;
                }
            }

            if !auto_reconnect {
                *running.lock() = false;
                return;
            }

            info!("Reconnecting in {} seconds...", reconnect_delay);
            tokio::time::sleep(std::time::Duration::from_secs(reconnect_delay as u64)).await;
        }
    }

    /// Run server mode
    async fn run_server(
        addr: SocketAddr,
        format: WsMessageFormat,
        namespace: String,
        event_tx: mpsc::Sender<BridgeEvent>,
        mut shutdown_rx: mpsc::Receiver<()>,
        running: Arc<Mutex<bool>>,
    ) {
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind WebSocket server: {}", e);
                let _ = event_tx
                    .send(BridgeEvent::Error(format!("Bind failed: {}", e)))
                    .await;
                return;
            }
        };

        info!("WebSocket server listening on {}", addr);
        *running.lock() = true;
        let _ = event_tx.send(BridgeEvent::Connected).await;

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, peer_addr)) => {
                            info!("WebSocket client connected: {}", peer_addr);

                            let format = format;
                            let namespace = namespace.clone();
                            let event_tx = event_tx.clone();

                            tokio::spawn(async move {
                                if let Ok(ws_stream) = accept_async(stream).await {
                                    let (_write, mut read) = ws_stream.split();

                                    while let Some(msg) = read.next().await {
                                        match msg {
                                            Ok(ws_msg) => {
                                                if let Some(clasp_msg) = Self::parse_message(&ws_msg, format, &namespace) {
                                                    let _ = event_tx.send(BridgeEvent::ToClasp(clasp_msg)).await;
                                                }
                                            }
                                            Err(e) => {
                                                debug!("WebSocket client error: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                }

                                info!("WebSocket client disconnected: {}", peer_addr);
                            });
                        }
                        Err(e) => {
                            error!("WebSocket accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("WebSocket server shutting down");
                    break;
                }
            }
        }

        *running.lock() = false;
        let _ = event_tx
            .send(BridgeEvent::Disconnected {
                reason: Some("Server stopped".to_string()),
            })
            .await;
    }
}

#[async_trait]
impl Bridge for WebSocketBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        let (event_tx, event_rx) = mpsc::channel(100);
        let (send_tx, send_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        self.send_tx = Some(send_tx);
        self.shutdown_tx = Some(shutdown_tx);

        let running = self.running.clone();
        let ws_config = self.ws_config.clone();

        match ws_config.mode {
            WsMode::Client => {
                tokio::spawn(Self::run_client(
                    ws_config.url,
                    ws_config.format,
                    ws_config.namespace,
                    ws_config.auto_reconnect,
                    ws_config.reconnect_delay_secs,
                    event_tx,
                    send_rx,
                    shutdown_rx,
                    running,
                ));
            }
            WsMode::Server => {
                let addr: SocketAddr = ws_config
                    .url
                    .parse()
                    .map_err(|e| BridgeError::Other(format!("Invalid address: {}", e)))?;

                tokio::spawn(Self::run_server(
                    addr,
                    ws_config.format,
                    ws_config.namespace,
                    event_tx,
                    shutdown_rx,
                    running,
                ));
            }
        }

        info!(
            "WebSocket bridge started in {:?} mode",
            self.ws_config.mode
        );
        Ok(event_rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        self.send_tx = None;
        info!("WebSocket bridge stopped");
        Ok(())
    }

    async fn send(&self, msg: Message) -> Result<()> {
        let send_tx = self
            .send_tx
            .as_ref()
            .ok_or_else(|| BridgeError::Other("Not connected".to_string()))?;

        if let Some(ws_msg) = Self::message_to_ws(&msg, self.ws_config.format) {
            send_tx
                .send(ws_msg)
                .await
                .map_err(|e| BridgeError::Other(format!("WebSocket send failed: {}", e)))?;
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.ws_config.namespace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WebSocketBridgeConfig::default();
        assert_eq!(config.mode, WsMode::Client);
        assert_eq!(config.namespace, "/ws");
    }

    #[test]
    fn test_message_formats() {
        let prefix = "/ws";

        // JSON text message
        let ws_msg = WsMessage::Text(r#"{"address": "/test", "value": 42}"#.to_string());
        let clasp = WebSocketBridge::parse_message(&ws_msg, WsMessageFormat::Json, prefix);
        assert!(clasp.is_some());

        // Plain text
        let ws_msg = WsMessage::Text("hello".to_string());
        let clasp = WebSocketBridge::parse_message(&ws_msg, WsMessageFormat::Json, prefix);
        assert!(clasp.is_some());

        // Binary
        let ws_msg = WsMessage::Binary(vec![1, 2, 3]);
        let clasp = WebSocketBridge::parse_message(&ws_msg, WsMessageFormat::Raw, prefix);
        assert!(clasp.is_some());
    }
}
