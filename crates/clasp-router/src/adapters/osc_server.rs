//! OSC Server Adapter
//!
//! Accepts OSC (Open Sound Control) messages via UDP and translates them to CLASP operations.
//! Unlike traditional OSC which is connectionless, this adapter tracks UDP sources as sessions
//! enabling bidirectional communication.
//!
//! ## Session Tracking
//!
//! Each unique UDP source address (IP:port) is tracked as a session. Sessions are cleaned up
//! after a configurable timeout of inactivity.
//!
//! ## Address Mapping
//!
//! OSC addresses are prefixed with the configured namespace (default: `/osc`):
//! - OSC `/synth/volume` → CLASP `/osc/synth/volume`
//!
//! ## Bidirectional Communication
//!
//! When CLASP messages are published to addresses matching OSC subscriptions,
//! they are converted back to OSC and sent to the subscribed UDP clients.

use bytes::Bytes;
use clasp_core::{codec, Message, SetMessage, SignalType, Value};
use dashmap::DashMap;
use parking_lot::RwLock;
use rosc::{OscBundle, OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};

use crate::error::{Result, RouterError};
use crate::session::{Session, SessionId};
use crate::state::RouterState;
use crate::subscription::{Subscription, SubscriptionManager};

/// OSC Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscServerConfig {
    /// Bind address for UDP socket (e.g., "0.0.0.0:8000")
    pub bind_addr: String,
    /// CLASP namespace prefix for OSC addresses (default: "/osc")
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// Session timeout in seconds (default: 30)
    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: u64,
    /// Auto-subscribe new sessions to all OSC addresses
    #[serde(default)]
    pub auto_subscribe: bool,
}

fn default_namespace() -> String {
    "/osc".to_string()
}

fn default_session_timeout() -> u64 {
    30
}

impl Default for OscServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8000".to_string(),
            namespace: "/osc".to_string(),
            session_timeout_secs: 30,
            auto_subscribe: false,
        }
    }
}

/// Internal OSC session tracking
struct OscSession {
    /// CLASP session ID
    clasp_session_id: SessionId,
    /// UDP source address
    peer_addr: SocketAddr,
    /// Last activity timestamp
    last_seen: RwLock<Instant>,
    /// Active subscriptions (CLASP patterns)
    subscriptions: RwLock<HashSet<String>>,
    /// Next subscription ID
    next_sub_id: AtomicU32,
}

impl OscSession {
    fn new(clasp_session_id: SessionId, peer_addr: SocketAddr) -> Self {
        Self {
            clasp_session_id,
            peer_addr,
            last_seen: RwLock::new(Instant::now()),
            subscriptions: RwLock::new(HashSet::new()),
            next_sub_id: AtomicU32::new(1),
        }
    }

    fn touch(&self) {
        *self.last_seen.write() = Instant::now();
    }

    fn idle_duration(&self) -> Duration {
        self.last_seen.read().elapsed()
    }

    fn next_subscription_id(&self) -> u32 {
        self.next_sub_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// OSC Server Adapter
///
/// Accepts OSC messages via UDP and translates them to CLASP operations.
/// Tracks UDP sources as sessions for bidirectional communication.
pub struct OscServerAdapter {
    config: OscServerConfig,
    /// Reference to router sessions
    sessions: Arc<DashMap<SessionId, Arc<Session>>>,
    /// Reference to router subscriptions
    subscriptions: Arc<SubscriptionManager>,
    /// Reference to router state
    state: Arc<RouterState>,
    /// OSC sessions by peer address
    osc_sessions: Arc<DashMap<SocketAddr, Arc<OscSession>>>,
    /// Running flag
    running: Arc<RwLock<bool>>,
    /// UDP socket for sending replies
    socket: Arc<RwLock<Option<Arc<UdpSocket>>>>,
}

impl OscServerAdapter {
    /// Create a new OSC server adapter
    pub fn new(
        config: OscServerConfig,
        sessions: Arc<DashMap<SessionId, Arc<Session>>>,
        subscriptions: Arc<SubscriptionManager>,
        state: Arc<RouterState>,
    ) -> Self {
        Self {
            config,
            sessions,
            subscriptions,
            state,
            osc_sessions: Arc::new(DashMap::new()),
            running: Arc::new(RwLock::new(false)),
            socket: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the OSC server
    pub async fn serve(&self) -> Result<()> {
        let socket = UdpSocket::bind(&self.config.bind_addr)
            .await
            .map_err(|e| RouterError::Transport(e.into()))?;

        let socket = Arc::new(socket);
        *self.socket.write() = Some(Arc::clone(&socket));

        info!("OSC server listening on {}", self.config.bind_addr);
        *self.running.write() = true;

        // Start session cleanup task
        self.start_cleanup_task();

        // Start outgoing message handler
        self.start_outgoing_handler(Arc::clone(&socket));

        let mut buf = vec![0u8; 65535];

        while *self.running.read() {
            match socket.recv_from(&mut buf).await {
                Ok((len, peer_addr)) => {
                    let data = &buf[..len];

                    // Get or create session for this peer
                    let osc_session = self.get_or_create_session(peer_addr);
                    osc_session.touch();

                    // Parse and handle OSC packet
                    match rosc::decoder::decode_udp(data) {
                        Ok((_, packet)) => {
                            self.handle_osc_packet(&osc_session, packet).await;
                        }
                        Err(e) => {
                            warn!("OSC decode error from {}: {}", peer_addr, e);
                        }
                    }
                }
                Err(e) => {
                    error!("OSC recv error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Get or create an OSC session for a peer address
    fn get_or_create_session(&self, peer_addr: SocketAddr) -> Arc<OscSession> {
        if let Some(session) = self.osc_sessions.get(&peer_addr) {
            return Arc::clone(session.value());
        }

        // Create new OSC session
        let osc_sender = OscTransportSender::new(peer_addr, Arc::clone(&self.socket));
        let clasp_session = Arc::new(Session::new(
            Arc::new(osc_sender),
            format!("osc:{}", peer_addr),
            vec!["osc".to_string()],
        ));
        let clasp_session_id = clasp_session.id.clone();
        self.sessions
            .insert(clasp_session_id.clone(), Arc::clone(&clasp_session));

        let osc_session = Arc::new(OscSession::new(clasp_session_id.clone(), peer_addr));

        // Auto-subscribe if configured
        if self.config.auto_subscribe {
            let pattern = format!("{}/**", self.config.namespace);
            let sub_id = osc_session.next_subscription_id();
            if let Ok(subscription) = Subscription::new(
                sub_id,
                clasp_session_id.clone(),
                &pattern,
                vec![],
                Default::default(),
            ) {
                self.subscriptions.add(subscription);
                osc_session.subscriptions.write().insert(pattern);
            }
        }

        self.osc_sessions
            .insert(peer_addr, Arc::clone(&osc_session));
        info!(
            "New OSC session from {} -> {}",
            peer_addr, clasp_session_id
        );

        osc_session
    }

    /// Handle an OSC packet
    async fn handle_osc_packet(&self, osc_session: &Arc<OscSession>, packet: OscPacket) {
        match packet {
            OscPacket::Message(msg) => {
                self.handle_osc_message(osc_session, msg).await;
            }
            OscPacket::Bundle(bundle) => {
                self.handle_osc_bundle(osc_session, bundle).await;
            }
        }
    }

    /// Handle an OSC message
    async fn handle_osc_message(&self, osc_session: &Arc<OscSession>, msg: OscMessage) {
        debug!(
            "OSC message from {}: {} {:?}",
            osc_session.peer_addr, msg.addr, msg.args
        );

        // Convert OSC address to CLASP address
        let clasp_address = format!("{}{}", self.config.namespace, msg.addr);

        // Convert OSC args to CLASP value
        let value = osc_args_to_value(&msg.args);

        // Apply to state
        let set_msg = SetMessage {
            address: clasp_address.clone(),
            value: value.clone(),
            revision: None,
            lock: false,
            unlock: false,
        };

        if let Ok(revision) = self.state.apply_set(&set_msg, &osc_session.clasp_session_id) {
            // Broadcast to CLASP subscribers
            let subscribers = self
                .subscriptions
                .find_subscribers(&clasp_address, Some(SignalType::Param));

            let mut updated_set = set_msg.clone();
            updated_set.revision = Some(revision);
            let broadcast_msg = Message::Set(updated_set);

            if let Ok(bytes) = codec::encode(&broadcast_msg) {
                for sub_session_id in subscribers {
                    // Don't send back to the OSC sender
                    if sub_session_id != osc_session.clasp_session_id {
                        if let Some(sub_session) = self.sessions.get(&sub_session_id) {
                            let _ = sub_session.try_send(bytes.clone());
                        }
                    }
                }
            }
        }
    }

    /// Handle an OSC bundle
    async fn handle_osc_bundle(&self, osc_session: &Arc<OscSession>, bundle: OscBundle) {
        for packet in bundle.content {
            Box::pin(self.handle_osc_packet(osc_session, packet)).await;
        }
    }

    /// Start background cleanup task for timed-out sessions
    fn start_cleanup_task(&self) {
        let osc_sessions = Arc::clone(&self.osc_sessions);
        let clasp_sessions = Arc::clone(&self.sessions);
        let subscriptions = Arc::clone(&self.subscriptions);
        let running = Arc::clone(&self.running);
        let timeout = Duration::from_secs(self.config.session_timeout_secs);

        tokio::spawn(async move {
            let check_interval = Duration::from_secs(10);

            loop {
                tokio::time::sleep(check_interval).await;

                if !*running.read() {
                    break;
                }

                // Find timed-out sessions
                let timed_out: Vec<SocketAddr> = osc_sessions
                    .iter()
                    .filter(|entry| entry.value().idle_duration() > timeout)
                    .map(|entry| *entry.key())
                    .collect();

                for peer_addr in timed_out {
                    if let Some((_, osc_session)) = osc_sessions.remove(&peer_addr) {
                        info!(
                            "OSC session {} timed out after {:?}",
                            peer_addr,
                            osc_session.idle_duration()
                        );
                        // Clean up CLASP session
                        clasp_sessions.remove(&osc_session.clasp_session_id);
                        subscriptions.remove_session(&osc_session.clasp_session_id);
                    }
                }
            }
        });
    }

    /// Start outgoing message handler
    fn start_outgoing_handler(&self, socket: Arc<UdpSocket>) {
        // This is handled through the OscTransportSender
        // which converts CLASP messages to OSC and sends them
    }

    /// Stop the OSC server
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Get connected session count
    pub fn session_count(&self) -> usize {
        self.osc_sessions.len()
    }
}

/// Convert OSC arguments to CLASP Value
fn osc_args_to_value(args: &[OscType]) -> Value {
    if args.is_empty() {
        return Value::Null;
    }

    if args.len() == 1 {
        return osc_type_to_value(&args[0]);
    }

    // Multiple args become an array
    Value::Array(args.iter().map(osc_type_to_value).collect())
}

/// Convert a single OSC type to CLASP Value
fn osc_type_to_value(osc: &OscType) -> Value {
    match osc {
        OscType::Int(i) => Value::Int(*i as i64),
        OscType::Float(f) => Value::Float(*f as f64),
        OscType::String(s) => Value::String(s.clone()),
        OscType::Blob(b) => Value::Bytes(b.clone()),
        OscType::Time(_) => Value::Null, // OSC time tags not directly mappable
        OscType::Long(l) => Value::Int(*l),
        OscType::Double(d) => Value::Float(*d),
        OscType::Char(c) => Value::String(c.to_string()),
        OscType::Color(c) => {
            // Convert RGBA color to array
            Value::Array(vec![
                Value::Int(c.red as i64),
                Value::Int(c.green as i64),
                Value::Int(c.blue as i64),
                Value::Int(c.alpha as i64),
            ])
        }
        OscType::Midi(m) => {
            // Convert MIDI to array [port, status, data1, data2]
            Value::Array(vec![
                Value::Int(m.port as i64),
                Value::Int(m.status as i64),
                Value::Int(m.data1 as i64),
                Value::Int(m.data2 as i64),
            ])
        }
        OscType::Bool(b) => Value::Bool(*b),
        OscType::Nil => Value::Null,
        OscType::Inf => Value::Float(f64::INFINITY),
        OscType::Array(arr) => Value::Array(arr.content.iter().map(osc_type_to_value).collect()),
    }
}

/// Convert CLASP Value to OSC arguments
fn value_to_osc_args(value: &Value) -> Vec<OscType> {
    match value {
        Value::Null => vec![OscType::Nil],
        Value::Bool(b) => vec![OscType::Bool(*b)],
        Value::Int(i) => {
            if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                vec![OscType::Int(*i as i32)]
            } else {
                vec![OscType::Long(*i)]
            }
        }
        Value::Float(f) => vec![OscType::Float(*f as f32)],
        Value::String(s) => vec![OscType::String(s.clone())],
        Value::Bytes(b) => vec![OscType::Blob(b.clone())],
        Value::Array(arr) => arr.iter().flat_map(value_to_osc_args).collect(),
        Value::Map(map) => {
            // Maps are serialized as JSON string
            if let Ok(json) = serde_json::to_string(map) {
                vec![OscType::String(json)]
            } else {
                vec![OscType::Nil]
            }
        }
    }
}

/// Transport sender implementation for OSC clients
struct OscTransportSender {
    peer_addr: SocketAddr,
    socket: Arc<RwLock<Option<Arc<UdpSocket>>>>,
}

impl OscTransportSender {
    fn new(peer_addr: SocketAddr, socket: Arc<RwLock<Option<Arc<UdpSocket>>>>) -> Self {
        Self { peer_addr, socket }
    }
}

#[async_trait::async_trait]
impl clasp_transport::TransportSender for OscTransportSender {
    async fn send(&self, data: Bytes) -> std::result::Result<(), clasp_transport::TransportError> {
        let socket = self.socket.read().clone();
        let socket = socket
            .ok_or_else(|| clasp_transport::TransportError::NotConnected)?;

        // Decode CLASP message and convert to OSC
        if let Ok((msg, _)) = codec::decode(&data) {
            if let Some(osc_data) = clasp_to_osc(&msg) {
                socket
                    .send_to(&osc_data, self.peer_addr)
                    .await
                    .map_err(|e| clasp_transport::TransportError::Io(e))?;
            }
        }
        Ok(())
    }

    fn try_send(&self, data: Bytes) -> std::result::Result<(), clasp_transport::TransportError> {
        // For UDP, we use blocking send in a blocking manner is not ideal
        // In practice, UDP sends are fast enough that this works
        let socket = self.socket.read().clone();
        if let Some(socket) = socket {
            if let Ok((msg, _)) = codec::decode(&data) {
                if let Some(osc_data) = clasp_to_osc(&msg) {
                    // Use try_send equivalent - for UDP this is effectively instant
                    let _ = socket.try_send_to(&osc_data, self.peer_addr);
                }
            }
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.socket.read().is_some()
    }

    async fn close(&self) -> std::result::Result<(), clasp_transport::TransportError> {
        // UDP socket doesn't need explicit close
        Ok(())
    }
}

/// Convert CLASP message to OSC packet bytes
fn clasp_to_osc(msg: &Message) -> Option<Vec<u8>> {
    let (address, value) = match msg {
        Message::Set(set) => (&set.address, &set.value),
        Message::Publish(pub_msg) => {
            if let Some(val) = &pub_msg.value {
                (&pub_msg.address, val)
            } else {
                return None;
            }
        }
        _ => return None,
    };

    // Strip /osc prefix to get OSC address
    let osc_addr = address
        .strip_prefix("/osc")
        .unwrap_or(address)
        .to_string();

    let args = value_to_osc_args(value);
    let msg = OscMessage {
        addr: osc_addr,
        args,
    };

    rosc::encoder::encode(&OscPacket::Message(msg)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_args_to_value() {
        // Single int
        let args = vec![OscType::Int(42)];
        let value = osc_args_to_value(&args);
        assert!(matches!(value, Value::Int(42)));

        // Multiple args become array
        let args = vec![OscType::Float(1.0), OscType::Float(2.0), OscType::Float(3.0)];
        let value = osc_args_to_value(&args);
        assert!(matches!(value, Value::Array(_)));
    }

    #[test]
    fn test_value_to_osc_args() {
        let value = Value::Float(42.5);
        let args = value_to_osc_args(&value);
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0], OscType::Float(f) if (f - 42.5).abs() < 0.001));
    }
}
