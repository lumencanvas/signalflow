//! MQTT Server Adapter
//!
//! Allows MQTT clients to connect directly to the CLASP router. The adapter
//! translates MQTT protocol operations to CLASP operations:
//!
//! | MQTT | CLASP |
//! |------|-------|
//! | CONNECT | Hello → Session |
//! | SUBSCRIBE `sensors/#` | Subscribe `/mqtt/sensors/**` |
//! | PUBLISH `sensors/temp` | Set `/mqtt/sensors/temp` |
//! | QoS 0 | Fire-and-forget |
//! | QoS 1 | With ACK |
//! | Username/Password | Token auth |

use bytes::{Bytes, BytesMut};
use clasp_core::{codec, Message, SetMessage, SignalType, Value};
use dashmap::DashMap;
use mqttbytes::v4::{
    ConnAck, ConnectReturnCode, Packet, PingResp, PubAck, Publish, SubAck, SubscribeReasonCode,
};
use mqttbytes::QoS;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::{Result, RouterError};
use crate::session::{Session, SessionId};
use crate::state::RouterState;
use crate::subscription::{Subscription, SubscriptionManager};

#[cfg(feature = "mqtts")]
use tokio_rustls::TlsAcceptor;

/// MQTT Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttServerConfig {
    /// Bind address (e.g., "0.0.0.0:1883")
    pub bind_addr: String,
    /// CLASP namespace prefix for MQTT topics (default: "/mqtt")
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// Require username/password authentication
    #[serde(default)]
    pub require_auth: bool,
    /// TLS configuration (for MQTTS on port 8883)
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    /// Maximum clients (0 = unlimited)
    #[serde(default)]
    pub max_clients: usize,
    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: u64,
}

fn default_namespace() -> String {
    "/mqtt".to_string()
}

fn default_session_timeout() -> u64 {
    300
}

impl Default for MqttServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:1883".to_string(),
            namespace: "/mqtt".to_string(),
            require_auth: false,
            tls: None,
            max_clients: 0,
            session_timeout_secs: 300,
        }
    }
}

/// TLS configuration for MQTTS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file (PEM format)
    pub cert_path: String,
    /// Path to private key file (PEM format)
    pub key_path: String,
}

/// Internal MQTT session tracking
struct MqttSession {
    /// CLASP session ID
    clasp_session_id: SessionId,
    /// MQTT client ID
    client_id: String,
    /// Peer address
    peer_addr: SocketAddr,
    /// Active MQTT subscriptions (topic filter -> subscription ID)
    mqtt_subscriptions: HashMap<String, u32>,
    /// Next subscription ID
    next_sub_id: AtomicU32,
    /// Last activity
    last_activity: RwLock<Instant>,
    /// Sender channel to MQTT client
    sender: mpsc::Sender<Bytes>,
}

impl MqttSession {
    fn new(
        clasp_session_id: SessionId,
        client_id: String,
        peer_addr: SocketAddr,
        sender: mpsc::Sender<Bytes>,
    ) -> Self {
        Self {
            clasp_session_id,
            client_id,
            peer_addr,
            mqtt_subscriptions: HashMap::new(),
            next_sub_id: AtomicU32::new(1),
            last_activity: RwLock::new(Instant::now()),
            sender,
        }
    }

    fn touch(&self) {
        *self.last_activity.write() = Instant::now();
    }

    fn idle_duration(&self) -> Duration {
        self.last_activity.read().elapsed()
    }

    fn next_subscription_id(&self) -> u32 {
        self.next_sub_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// MQTT Server Adapter
///
/// Accepts MQTT client connections and translates them to CLASP operations.
pub struct MqttServerAdapter {
    config: MqttServerConfig,
    /// Reference to router sessions
    sessions: Arc<DashMap<SessionId, Arc<Session>>>,
    /// Reference to router subscriptions
    subscriptions: Arc<SubscriptionManager>,
    /// Reference to router state
    state: Arc<RouterState>,
    /// MQTT sessions by client_id
    mqtt_sessions: Arc<DashMap<String, Arc<MqttSession>>>,
    /// Running flag
    running: Arc<RwLock<bool>>,
    /// TLS acceptor (if configured)
    #[cfg(feature = "mqtts")]
    tls_acceptor: Option<TlsAcceptor>,
}

impl MqttServerAdapter {
    /// Create a new MQTT server adapter
    pub fn new(
        config: MqttServerConfig,
        sessions: Arc<DashMap<SessionId, Arc<Session>>>,
        subscriptions: Arc<SubscriptionManager>,
        state: Arc<RouterState>,
    ) -> Self {
        Self {
            config,
            sessions,
            subscriptions,
            state,
            mqtt_sessions: Arc::new(DashMap::new()),
            running: Arc::new(RwLock::new(false)),
            #[cfg(feature = "mqtts")]
            tls_acceptor: None,
        }
    }

    /// Start the MQTT server
    pub async fn serve(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr)
            .await
            .map_err(|e| RouterError::Transport(e.into()))?;

        info!("MQTT server listening on {}", self.config.bind_addr);
        *self.running.write() = true;

        // Start session cleanup task
        self.start_cleanup_task();

        while *self.running.read() {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    // Check max clients
                    if self.config.max_clients > 0
                        && self.mqtt_sessions.len() >= self.config.max_clients
                    {
                        warn!(
                            "Rejecting MQTT connection from {}: max clients reached",
                            peer_addr
                        );
                        continue;
                    }

                    info!("MQTT connection from {}", peer_addr);
                    self.spawn_connection_handler(stream, peer_addr);
                }
                Err(e) => {
                    error!("MQTT accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Start background cleanup task for timed-out sessions
    fn start_cleanup_task(&self) {
        let mqtt_sessions = Arc::clone(&self.mqtt_sessions);
        let clasp_sessions = Arc::clone(&self.sessions);
        let subscriptions = Arc::clone(&self.subscriptions);
        let running = Arc::clone(&self.running);
        let timeout = Duration::from_secs(self.config.session_timeout_secs);

        tokio::spawn(async move {
            let check_interval = Duration::from_secs(30);

            loop {
                tokio::time::sleep(check_interval).await;

                if !*running.read() {
                    break;
                }

                // Find timed-out sessions
                let timed_out: Vec<String> = mqtt_sessions
                    .iter()
                    .filter(|entry| entry.value().idle_duration() > timeout)
                    .map(|entry| entry.key().clone())
                    .collect();

                for client_id in timed_out {
                    if let Some((_, mqtt_session)) = mqtt_sessions.remove(&client_id) {
                        info!(
                            "MQTT session {} timed out after {:?}",
                            client_id,
                            mqtt_session.idle_duration()
                        );
                        // Clean up CLASP session
                        clasp_sessions.remove(&mqtt_session.clasp_session_id);
                        subscriptions.remove_session(&mqtt_session.clasp_session_id);
                    }
                }
            }
        });
    }

    /// Spawn a connection handler for an MQTT client
    fn spawn_connection_handler(&self, stream: TcpStream, peer_addr: SocketAddr) {
        let config = self.config.clone();
        let sessions = Arc::clone(&self.sessions);
        let subscriptions = Arc::clone(&self.subscriptions);
        let state = Arc::clone(&self.state);
        let mqtt_sessions = Arc::clone(&self.mqtt_sessions);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            if let Err(e) = handle_mqtt_connection(
                stream,
                peer_addr,
                config,
                sessions,
                subscriptions,
                state,
                mqtt_sessions,
                running,
            )
            .await
            {
                debug!("MQTT connection {} ended: {}", peer_addr, e);
            }
        });
    }

    /// Stop the MQTT server
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Get connected client count
    pub fn client_count(&self) -> usize {
        self.mqtt_sessions.len()
    }
}

/// Handle an individual MQTT connection
async fn handle_mqtt_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    config: MqttServerConfig,
    clasp_sessions: Arc<DashMap<SessionId, Arc<Session>>>,
    subscriptions: Arc<SubscriptionManager>,
    state: Arc<RouterState>,
    mqtt_sessions: Arc<DashMap<String, Arc<MqttSession>>>,
    running: Arc<RwLock<bool>>,
) -> Result<()> {
    let mut read_buf = BytesMut::with_capacity(4096);

    // Create channel for sending data to this client
    let (tx, mut rx) = mpsc::channel::<Bytes>(100);

    // Wait for CONNECT packet
    let connect = loop {
        if !*running.read() {
            return Ok(());
        }

        // Read data
        let n = stream.read_buf(&mut read_buf).await?;
        if n == 0 {
            return Err(RouterError::Protocol("Connection closed".into()));
        }

        // Try to parse CONNECT packet
        match mqttbytes::v4::read(&mut read_buf, 65535) {
            Ok(Packet::Connect(connect)) => break connect,
            Ok(other) => {
                warn!("Expected CONNECT, got {:?}", other);
                return Err(RouterError::Protocol("Expected CONNECT packet".into()));
            }
            Err(mqttbytes::Error::InsufficientBytes(_)) => {
                // Need more data
                continue;
            }
            Err(e) => {
                return Err(RouterError::Protocol(format!("MQTT parse error: {}", e)));
            }
        }
    };

    let client_id = connect.client_id.clone();
    info!("MQTT CONNECT from {} (client_id: {})", peer_addr, client_id);

    // Validate credentials if required
    if config.require_auth {
        if connect.login.is_none() {
            let connack = ConnAck {
                session_present: false,
                code: ConnectReturnCode::BadUserNamePassword,
            };
            let mut buf = BytesMut::new();
            connack.write(&mut buf)?;
            stream.write_all(&buf).await?;
            return Err(RouterError::Auth("Authentication required".into()));
        }
        // TODO: Validate username/password against token validator
    }

    // Create CLASP session (using a transport sender that writes to our channel)
    let mqtt_sender = MqttTransportSender::new(tx.clone());
    let clasp_session = Arc::new(Session::new(
        Arc::new(mqtt_sender),
        format!("mqtt:{}", client_id),
        vec!["mqtt".to_string()],
    ));
    let clasp_session_id = clasp_session.id.clone();
    clasp_sessions.insert(clasp_session_id.clone(), clasp_session);

    // Create MQTT session
    let mqtt_session = Arc::new(MqttSession::new(
        clasp_session_id.clone(),
        client_id.clone(),
        peer_addr,
        tx,
    ));
    mqtt_sessions.insert(client_id.clone(), Arc::clone(&mqtt_session));

    // Send CONNACK
    let connack = ConnAck {
        session_present: false,
        code: ConnectReturnCode::Success,
    };
    let mut buf = BytesMut::new();
    connack.write(&mut buf)?;
    stream.write_all(&buf).await?;

    info!("MQTT session established: {} -> {}", client_id, clasp_session_id);

    // Main loop: handle incoming packets and outgoing messages
    loop {
        if !*running.read() {
            break;
        }

        tokio::select! {
            // Read from MQTT client
            result = stream.read_buf(&mut read_buf) => {
                match result {
                    Ok(0) => {
                        info!("MQTT client {} disconnected", client_id);
                        break;
                    }
                    Ok(_) => {
                        mqtt_session.touch();

                        // Process all complete packets in buffer
                        loop {
                            match mqttbytes::v4::read(&mut read_buf, 65535) {
                                Ok(packet) => {
                                    if let Err(e) = handle_mqtt_packet(
                                        &packet,
                                        &mqtt_session,
                                        &config,
                                        &subscriptions,
                                        &state,
                                        &clasp_sessions,
                                        &mut stream,
                                    ).await {
                                        warn!("Error handling MQTT packet: {}", e);
                                    }

                                    // Check for DISCONNECT
                                    if matches!(packet, Packet::Disconnect) {
                                        info!("MQTT client {} sent DISCONNECT", client_id);
                                        break;
                                    }
                                }
                                Err(mqttbytes::Error::InsufficientBytes(_)) => {
                                    // Need more data
                                    break;
                                }
                                Err(e) => {
                                    warn!("MQTT parse error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("MQTT read error: {}", e);
                        break;
                    }
                }
            }

            // Send outgoing messages to MQTT client
            Some(data) = rx.recv() => {
                if let Err(e) = stream.write_all(&data).await {
                    error!("MQTT write error: {}", e);
                    break;
                }
            }
        }
    }

    // Cleanup
    mqtt_sessions.remove(&client_id);
    clasp_sessions.remove(&clasp_session_id);
    subscriptions.remove_session(&clasp_session_id);

    info!("MQTT session {} cleaned up", client_id);
    Ok(())
}

/// Handle a single MQTT packet
async fn handle_mqtt_packet(
    packet: &Packet,
    mqtt_session: &Arc<MqttSession>,
    config: &MqttServerConfig,
    subscriptions: &Arc<SubscriptionManager>,
    state: &Arc<RouterState>,
    clasp_sessions: &Arc<DashMap<SessionId, Arc<Session>>>,
    stream: &mut TcpStream,
) -> Result<()> {
    match packet {
        Packet::Subscribe(subscribe) => {
            debug!(
                "MQTT SUBSCRIBE from {}: {:?}",
                mqtt_session.client_id, subscribe.filters
            );

            let mut return_codes = Vec::new();

            for filter in &subscribe.filters {
                let topic_filter = &filter.path;
                let qos = filter.qos;

                // Convert MQTT topic filter to CLASP pattern
                let clasp_pattern = mqtt_topic_to_clasp_pattern(&config.namespace, topic_filter);

                // Create CLASP subscription
                let sub_id = mqtt_session.next_subscription_id();
                match Subscription::new(
                    sub_id,
                    mqtt_session.clasp_session_id.clone(),
                    &clasp_pattern,
                    vec![], // All signal types
                    Default::default(),
                ) {
                    Ok(subscription) => {
                        subscriptions.add(subscription);
                        return_codes.push(SubscribeReasonCode::Success(qos));
                        debug!(
                            "MQTT subscription {} -> CLASP pattern {}",
                            topic_filter, clasp_pattern
                        );

                        // Send current state matching this pattern as retained messages
                        let snapshot = state.snapshot(&clasp_pattern);
                        for param in snapshot.params {
                            let mqtt_topic =
                                clasp_address_to_mqtt_topic(&config.namespace, &param.address);
                            let payload = value_to_mqtt_payload(&param.value);

                            let publish = Publish::new(&mqtt_topic, QoS::AtMostOnce, payload);
                            let mut buf = BytesMut::new();
                            publish.write(&mut buf)?;
                            stream.write_all(&buf).await?;
                        }
                    }
                    Err(e) => {
                        warn!("Invalid MQTT subscription pattern: {}", e);
                        return_codes.push(SubscribeReasonCode::Failure);
                    }
                }
            }

            // Send SUBACK
            let suback = SubAck {
                pkid: subscribe.pkid,
                return_codes,
            };
            let mut buf = BytesMut::new();
            suback.write(&mut buf)?;
            stream.write_all(&buf).await?;
        }

        Packet::Publish(publish) => {
            debug!(
                "MQTT PUBLISH from {}: {} ({} bytes)",
                mqtt_session.client_id,
                publish.topic,
                publish.payload.len()
            );

            // Convert MQTT topic to CLASP address
            let clasp_address = mqtt_topic_to_clasp_address(&config.namespace, &publish.topic);

            // Parse payload to CLASP value
            let value = mqtt_payload_to_value(&publish.payload);

            // Apply to state
            let set_msg = SetMessage {
                address: clasp_address.clone(),
                value: value.clone(),
                revision: None,
                lock: false,
                unlock: false,
            };

            if let Ok(revision) = state.apply_set(&set_msg, &mqtt_session.clasp_session_id) {
                // Broadcast to CLASP subscribers
                let subscribers =
                    subscriptions.find_subscribers(&clasp_address, Some(SignalType::Param));

                let mut updated_set = set_msg.clone();
                updated_set.revision = Some(revision);
                let broadcast_msg = Message::Set(updated_set);

                if let Ok(bytes) = codec::encode(&broadcast_msg) {
                    for sub_session_id in subscribers {
                        // Don't send back to the MQTT sender
                        if sub_session_id != mqtt_session.clasp_session_id {
                            if let Some(sub_session) = clasp_sessions.get(&sub_session_id) {
                                let _ = sub_session.try_send(bytes.clone());
                            }
                        }
                    }
                }

                // Send PUBACK for QoS 1
                if publish.qos == QoS::AtLeastOnce {
                    let puback = PubAck { pkid: publish.pkid };
                    let mut buf = BytesMut::new();
                    puback.write(&mut buf)?;
                    stream.write_all(&buf).await?;
                }
            }
        }

        Packet::Unsubscribe(unsubscribe) => {
            debug!(
                "MQTT UNSUBSCRIBE from {}: {:?}",
                mqtt_session.client_id, unsubscribe.topics
            );

            // Remove subscriptions
            // Note: We'd need to track which subscription IDs map to which topics
            // For now, this is a simplified implementation

            // Send UNSUBACK
            let unsuback = mqttbytes::v4::UnsubAck {
                pkid: unsubscribe.pkid,
            };
            let mut buf = BytesMut::new();
            unsuback.write(&mut buf)?;
            stream.write_all(&buf).await?;
        }

        Packet::PingReq => {
            debug!("MQTT PINGREQ from {}", mqtt_session.client_id);
            let pingresp = PingResp;
            let mut buf = BytesMut::new();
            pingresp.write(&mut buf)?;
            stream.write_all(&buf).await?;
        }

        Packet::Disconnect => {
            info!("MQTT DISCONNECT from {}", mqtt_session.client_id);
            // Handled in main loop
        }

        other => {
            debug!("Unhandled MQTT packet: {:?}", other);
        }
    }

    Ok(())
}

/// Convert MQTT topic filter to CLASP pattern
///
/// MQTT wildcards:
/// - `+` matches a single level (CLASP: `*`)
/// - `#` matches multiple levels (CLASP: `**`)
fn mqtt_topic_to_clasp_pattern(namespace: &str, topic: &str) -> String {
    let clasp_path = topic
        .replace('+', "*")
        .replace('#', "**")
        .replace('/', "/");
    format!("{}/{}", namespace, clasp_path)
}

/// Convert MQTT topic to CLASP address (for publish)
fn mqtt_topic_to_clasp_address(namespace: &str, topic: &str) -> String {
    format!("{}/{}", namespace, topic)
}

/// Convert CLASP address to MQTT topic
fn clasp_address_to_mqtt_topic(namespace: &str, address: &str) -> String {
    address
        .strip_prefix(namespace)
        .unwrap_or(address)
        .trim_start_matches('/')
        .to_string()
}

/// Convert MQTT payload to CLASP Value
fn mqtt_payload_to_value(payload: &[u8]) -> Value {
    if let Ok(text) = std::str::from_utf8(payload) {
        // Try JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            return json_to_clasp_value(json);
        }
        // Try as number
        if let Ok(f) = text.parse::<f64>() {
            return Value::Float(f);
        }
        // Try as bool
        match text {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            _ => {}
        }
        // Return as string
        return Value::String(text.to_string());
    }
    Value::Bytes(payload.to_vec())
}

/// Convert serde JSON to CLASP Value
fn json_to_clasp_value(json: serde_json::Value) -> Value {
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
            Value::Array(arr.into_iter().map(json_to_clasp_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, Value> = obj
                .into_iter()
                .map(|(k, v)| (k, json_to_clasp_value(v)))
                .collect();
            Value::Map(map)
        }
    }
}

/// Convert CLASP Value to MQTT payload bytes
fn value_to_mqtt_payload(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => b"null".to_vec(),
        Value::Bool(b) => (if *b { "true" } else { "false" }).as_bytes().to_vec(),
        Value::Int(i) => i.to_string().into_bytes(),
        Value::Float(f) => f.to_string().into_bytes(),
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Bytes(b) => b.clone(),
        Value::Array(_) | Value::Map(_) => {
            serde_json::to_vec(value).unwrap_or_else(|_| b"null".to_vec())
        }
    }
}

/// Transport sender implementation for MQTT clients
struct MqttTransportSender {
    tx: mpsc::Sender<Bytes>,
}

impl MqttTransportSender {
    fn new(tx: mpsc::Sender<Bytes>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl clasp_transport::TransportSender for MqttTransportSender {
    async fn send(&self, data: Bytes) -> std::result::Result<(), clasp_transport::TransportError> {
        // MQTT clients receive CLASP messages as MQTT PUBLISH packets
        // We need to convert the CLASP message to MQTT format
        // For now, this is a placeholder - actual implementation would parse
        // the CLASP message and convert to MQTT PUBLISH

        // Decode CLASP message
        if let Ok((msg, _)) = codec::decode(&data) {
            if let Some(mqtt_data) = clasp_to_mqtt_publish(&msg) {
                self.tx
                    .send(mqtt_data)
                    .await
                    .map_err(|e| clasp_transport::TransportError::SendFailed(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn try_send(&self, data: Bytes) -> std::result::Result<(), clasp_transport::TransportError> {
        if let Ok((msg, _)) = codec::decode(&data) {
            if let Some(mqtt_data) = clasp_to_mqtt_publish(&msg) {
                self.tx
                    .try_send(mqtt_data)
                    .map_err(|e| clasp_transport::TransportError::SendFailed(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        !self.tx.is_closed()
    }

    async fn close(&self) -> std::result::Result<(), clasp_transport::TransportError> {
        // Channel will be closed when dropped
        Ok(())
    }
}

/// Convert CLASP message to MQTT PUBLISH packet bytes
fn clasp_to_mqtt_publish(msg: &Message) -> Option<Bytes> {
    let (address, value) = match msg {
        Message::Set(set) => (&set.address, &set.value),
        Message::Publish(pub_msg) => {
            if let Some(val) = &pub_msg.value {
                (&pub_msg.address, val)
            } else {
                return None;
            }
        }
        Message::Snapshot(snapshot) => {
            // For snapshots, we'd need to send multiple PUBLISH packets
            // This is handled separately
            return None;
        }
        _ => return None,
    };

    // Strip /mqtt prefix to get MQTT topic
    let topic = address
        .strip_prefix("/mqtt/")
        .unwrap_or(address.trim_start_matches('/'));
    let payload = value_to_mqtt_payload(value);

    let publish = Publish::new(topic, QoS::AtMostOnce, payload);
    let mut buf = BytesMut::new();
    if publish.write(&mut buf).is_ok() {
        Some(buf.freeze())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_to_clasp_pattern() {
        assert_eq!(
            mqtt_topic_to_clasp_pattern("/mqtt", "sensors/temp"),
            "/mqtt/sensors/temp"
        );
        assert_eq!(
            mqtt_topic_to_clasp_pattern("/mqtt", "sensors/+/temp"),
            "/mqtt/sensors/*/temp"
        );
        assert_eq!(
            mqtt_topic_to_clasp_pattern("/mqtt", "sensors/#"),
            "/mqtt/sensors/**"
        );
    }

    #[test]
    fn test_address_to_topic() {
        assert_eq!(
            clasp_address_to_mqtt_topic("/mqtt", "/mqtt/sensors/temp"),
            "sensors/temp"
        );
    }

    #[test]
    fn test_payload_parsing() {
        // Number
        let value = mqtt_payload_to_value(b"42.5");
        assert!(matches!(value, Value::Float(f) if (f - 42.5).abs() < 0.001));

        // Boolean
        let value = mqtt_payload_to_value(b"true");
        assert!(matches!(value, Value::Bool(true)));

        // JSON
        let value = mqtt_payload_to_value(b"{\"temp\": 25}");
        assert!(matches!(value, Value::Map(_)));

        // String
        let value = mqtt_payload_to_value(b"hello world");
        assert!(matches!(value, Value::String(s) if s == "hello world"));
    }
}
