//! P2P connection manager for native CLASP clients
//!
//! This module provides WebRTC peer-to-peer connectivity:
//! - P2PManager - manages multiple peer connections
//! - P2PConnection - wrapper for a single WebRTC peer connection
//! - Signaling via PUBLISH messages through the router

use bytes::Bytes;
use clasp_core::{
    signal_address, Message, P2PAnnounce, P2PConfig, P2PConnectionState, P2PSignal, PublishMessage,
    RoutingMode, SignalType, Value, P2P_ANNOUNCE, P2P_SIGNAL_PREFIX,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[cfg(feature = "p2p")]
use clasp_transport::{WebRtcConfig, WebRtcTransport};

use crate::error::{ClientError, Result};

/// Callback for P2P events
pub type P2PEventCallback = Box<dyn Fn(P2PEvent) + Send + Sync>;

/// P2P connection events
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// A peer announced its P2P capability
    PeerAnnounced {
        session_id: String,
        features: Vec<String>,
    },
    /// P2P connection established with a peer
    Connected { peer_session_id: String },
    /// P2P connection failed
    ConnectionFailed {
        peer_session_id: String,
        reason: String,
    },
    /// P2P connection closed
    Disconnected {
        peer_session_id: String,
        reason: Option<String>,
    },
    /// Data received from peer via P2P
    Data {
        peer_session_id: String,
        data: Bytes,
        reliable: bool,
    },
}

/// P2P peer connection wrapper
#[cfg(feature = "p2p")]
pub struct P2PConnection {
    /// Remote peer's session ID
    pub peer_session_id: String,
    /// Correlation ID for this connection
    pub correlation_id: String,
    /// Connection state
    pub state: P2PConnectionState,
    /// WebRTC transport (once connected)
    transport: Option<WebRtcTransport>,
    /// Pending ICE candidates (received before remote description set)
    pending_candidates: Vec<String>,
}

#[cfg(feature = "p2p")]
impl P2PConnection {
    /// Create a new P2P connection
    fn new(peer_session_id: String, correlation_id: String) -> Self {
        Self {
            peer_session_id,
            correlation_id,
            state: P2PConnectionState::Disconnected,
            transport: None,
            pending_candidates: Vec::new(),
        }
    }

    /// Add a pending ICE candidate
    fn add_pending_candidate(&mut self, candidate: String) {
        self.pending_candidates.push(candidate);
    }

    /// Get and clear pending candidates
    fn take_pending_candidates(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_candidates)
    }
}

/// P2P connection manager
///
/// Manages multiple WebRTC peer connections and handles signaling
/// through the CLASP router.
pub struct P2PManager {
    /// Our session ID
    session_id: RwLock<Option<String>>,
    /// P2P configuration
    config: P2PConfig,
    /// Active peer connections
    #[cfg(feature = "p2p")]
    connections: Arc<DashMap<String, P2PConnection>>,
    #[cfg(not(feature = "p2p"))]
    connections: Arc<DashMap<String, ()>>,
    /// Known P2P-capable peers
    known_peers: Arc<DashMap<String, Vec<String>>>,
    /// Event callback
    event_callback: RwLock<Option<P2PEventCallback>>,
    /// Channel for sending outgoing signaling messages
    signal_tx: mpsc::Sender<Message>,
    /// Routing mode
    routing_mode: RwLock<RoutingMode>,
}

impl P2PManager {
    /// Create a new P2P manager
    ///
    /// # Arguments
    /// * `config` - P2P configuration
    /// * `signal_tx` - Channel for sending outgoing messages to the server
    pub fn new(config: P2PConfig, signal_tx: mpsc::Sender<Message>) -> Self {
        Self {
            session_id: RwLock::new(None),
            config,
            connections: Arc::new(DashMap::new()),
            known_peers: Arc::new(DashMap::new()),
            event_callback: RwLock::new(None),
            signal_tx,
            routing_mode: RwLock::new(RoutingMode::PreferP2P),
        }
    }

    /// Set the session ID (called after connection to server)
    pub fn set_session_id(&self, session_id: String) {
        *self.session_id.write() = Some(session_id);
    }

    /// Get our session ID
    pub fn session_id(&self) -> Option<String> {
        self.session_id.read().clone()
    }

    /// Set the event callback
    pub fn on_event<F>(&self, callback: F)
    where
        F: Fn(P2PEvent) + Send + Sync + 'static,
    {
        *self.event_callback.write() = Some(Box::new(callback));
    }

    /// Set the routing mode
    pub fn set_routing_mode(&self, mode: RoutingMode) {
        *self.routing_mode.write() = mode;
    }

    /// Get the current routing mode
    pub fn routing_mode(&self) -> RoutingMode {
        *self.routing_mode.read()
    }

    /// Announce our P2P capability to the network
    pub async fn announce(&self) -> Result<()> {
        let session_id = self.session_id().ok_or(ClientError::NotConnected)?;

        let announce = P2PAnnounce {
            session_id,
            p2p_capable: true,
            features: vec![
                "webrtc".to_string(),
                "reliable".to_string(),
                "unreliable".to_string(),
            ],
        };

        let payload =
            serde_json::to_value(&announce).map_err(|e| ClientError::Other(e.to_string()))?;

        let msg = Message::Publish(PublishMessage {
            address: P2P_ANNOUNCE.to_string(),
            signal: Some(SignalType::Event),
            value: None,
            payload: Some(value_from_json(payload)),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
            timeline: None,
        });

        self.signal_tx
            .send(msg)
            .await
            .map_err(|e| ClientError::SendFailed(e.to_string()))?;

        info!("P2P capability announced");
        Ok(())
    }

    /// Initiate a P2P connection to a peer
    #[cfg(feature = "p2p")]
    pub async fn connect_to_peer(self: &Arc<Self>, peer_session_id: &str) -> Result<()> {
        let our_session_id = self.session_id().ok_or(ClientError::NotConnected)?;

        // Generate correlation ID for this connection
        let correlation_id = format!(
            "{}-{}-{}",
            our_session_id,
            peer_session_id,
            uuid::Uuid::new_v4()
        );

        // Create connection entry
        let mut connection =
            P2PConnection::new(peer_session_id.to_string(), correlation_id.clone());
        connection.state = P2PConnectionState::Connecting;

        // Create WebRTC transport as offerer
        let webrtc_config = WebRtcConfig {
            ice_servers: self.config.ice_servers.clone(),
            unreliable_channel: true,
            reliable_channel: true,
        };

        let (transport, sdp_offer) = WebRtcTransport::new_offerer_with_config(webrtc_config)
            .await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        // Set up connection monitoring for offerer (before storing transport)
        let p2p_manager = Arc::clone(self);
        let peer_id = peer_session_id.to_string();
        info!("Setting up connection callback for offerer to peer {}", peer_id);
        transport.on_connection_ready(move || {
            info!("Connection callback invoked for offerer to peer {}", peer_id);
            let p2p = Arc::clone(&p2p_manager);
            let peer = peer_id.clone();
            tokio::spawn(async move {
                info!("Calling mark_connected for peer {}", peer);
                if let Err(e) = p2p.mark_connected(&peer).await {
                    warn!("Failed to mark connected: {}", e);
                } else {
                    info!("Successfully marked connected for peer {}", peer);
                }
            });
        });

        // Set up ICE candidate handler for offerer
        let p2p_manager_ice = Arc::clone(self);
        let peer_id_ice = peer_session_id.to_string();
        let correlation_id_ice = correlation_id.clone();
        transport.on_ice_candidate(move |candidate_json| {
            debug!("ICE candidate generated for offerer to peer {}: {}", peer_id_ice, candidate_json);
            let p2p = Arc::clone(&p2p_manager_ice);
            let peer = peer_id_ice.clone();
            let candidate = candidate_json.clone();
            let corr_id = correlation_id_ice.clone();
            tokio::spawn(async move {
                let signal = P2PSignal::IceCandidate {
                    from: p2p.session_id().unwrap_or_default(),
                    candidate,
                    correlation_id: corr_id,
                };
                if let Err(e) = p2p.send_signal(&peer, signal).await {
                    warn!("Failed to send ICE candidate: {}", e);
                }
            });
        });

        connection.transport = Some(transport);

        // Store connection
        self.connections
            .insert(peer_session_id.to_string(), connection);

        // Send offer via signaling
        let signal = P2PSignal::Offer {
            from: our_session_id,
            sdp: sdp_offer,
            correlation_id,
        };

        self.send_signal(peer_session_id, signal).await?;

        info!("P2P connection initiated to {}", peer_session_id);
        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    pub async fn connect_to_peer(&self, _peer_session_id: &str) -> Result<()> {
        Err(ClientError::Other(
            "P2P feature not enabled. Compile with --features p2p".to_string(),
        ))
    }

    /// Handle incoming signaling message
    pub async fn handle_signal(self: &Arc<Self>, address: &str, payload: &Value) -> Result<()> {
        // Extract target session from address (for signals meant for us)
        if !address.starts_with(P2P_SIGNAL_PREFIX) {
            return Ok(());
        }

        // Check if this signal is for us
        let our_session = self.session_id();
        if let Some(ref session) = our_session {
            let expected_address = format!("{}{}", P2P_SIGNAL_PREFIX, session);
            if address != expected_address {
                // Not for us, ignore
                return Ok(());
            }
        } else {
            return Ok(());
        }

        // Parse the signal
        let json = value_to_json(payload);
        let signal: P2PSignal =
            serde_json::from_value(json).map_err(|e| ClientError::Other(e.to_string()))?;

        match signal {
            P2PSignal::Offer {
                from,
                sdp,
                correlation_id,
            } => {
                self.handle_offer(&from, &sdp, &correlation_id).await?;
            }
            P2PSignal::Answer {
                from,
                sdp,
                correlation_id,
            } => {
                self.handle_answer(&from, &sdp, &correlation_id).await?;
            }
            P2PSignal::IceCandidate {
                from,
                candidate,
                correlation_id,
            } => {
                self.handle_ice_candidate(&from, &candidate, &correlation_id)
                    .await?;
            }
            P2PSignal::Connected {
                from,
                correlation_id,
            } => {
                self.handle_connected(&from, &correlation_id).await?;
            }
            P2PSignal::Disconnected {
                from,
                correlation_id,
                reason,
            } => {
                self.handle_disconnected(&from, &correlation_id, reason.as_deref())
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle incoming P2P announce
    pub fn handle_announce(&self, payload: &Value) {
        let json = value_to_json(payload);
        if let Ok(announce) = serde_json::from_value::<P2PAnnounce>(json) {
            // Don't track ourselves
            if Some(&announce.session_id) == self.session_id.read().as_ref() {
                return;
            }

            // Store the peer's capabilities
            self.known_peers
                .insert(announce.session_id.clone(), announce.features.clone());

            // Notify via callback
            if let Some(callback) = self.event_callback.read().as_ref() {
                callback(P2PEvent::PeerAnnounced {
                    session_id: announce.session_id,
                    features: announce.features,
                });
            }
        }
    }

    /// Get list of known P2P-capable peers
    pub fn known_peers(&self) -> Vec<String> {
        self.known_peers.iter().map(|e| e.key().clone()).collect()
    }

    /// Check if a peer is connected via P2P
    #[cfg(feature = "p2p")]
    pub fn is_peer_connected(&self, peer_session_id: &str) -> bool {
        self.connections
            .get(peer_session_id)
            .map(|c| c.state == P2PConnectionState::Connected)
            .unwrap_or(false)
    }

    #[cfg(not(feature = "p2p"))]
    pub fn is_peer_connected(&self, _peer_session_id: &str) -> bool {
        false
    }

    /// Disconnect from a peer
    #[cfg(feature = "p2p")]
    pub async fn disconnect_peer(&self, peer_session_id: &str) -> Result<()> {
        if let Some((_, connection)) = self.connections.remove(peer_session_id) {
            let our_session_id = self.session_id().ok_or(ClientError::NotConnected)?;

            // Send disconnect signal
            let signal = P2PSignal::Disconnected {
                from: our_session_id,
                correlation_id: connection.correlation_id,
                reason: Some("User requested disconnect".to_string()),
            };

            self.send_signal(peer_session_id, signal).await?;

            // Notify via callback
            if let Some(callback) = self.event_callback.read().as_ref() {
                callback(P2PEvent::Disconnected {
                    peer_session_id: peer_session_id.to_string(),
                    reason: Some("User requested disconnect".to_string()),
                });
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    pub async fn disconnect_peer(&self, _peer_session_id: &str) -> Result<()> {
        Ok(())
    }

    // =========================================================================
    // Internal signal handlers
    // =========================================================================

    #[cfg(feature = "p2p")]
    async fn handle_offer(self: &Arc<Self>, from: &str, sdp: &str, correlation_id: &str) -> Result<()> {
        let our_session_id = self.session_id().ok_or(ClientError::NotConnected)?;

        info!("Received P2P offer from {}", from);

        // Create answerer transport
        let webrtc_config = WebRtcConfig {
            ice_servers: self.config.ice_servers.clone(),
            unreliable_channel: true,
            reliable_channel: true,
        };

        let (transport, sdp_answer) = WebRtcTransport::new_answerer_with_config(sdp, webrtc_config)
            .await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        // Set up connection monitoring for answerer (before storing transport)
        let p2p_manager = Arc::clone(self);
        let peer_id = from.to_string();
        info!("Setting up connection callback for answerer from peer {}", peer_id);
        transport.on_connection_ready(move || {
            info!("Connection callback invoked for answerer from peer {}", peer_id);
            let p2p = Arc::clone(&p2p_manager);
            let peer = peer_id.clone();
            tokio::spawn(async move {
                info!("Calling mark_connected for peer {}", peer);
                if let Err(e) = p2p.mark_connected(&peer).await {
                    warn!("Failed to mark connected: {}", e);
                } else {
                    info!("Successfully marked connected for peer {}", peer);
                }
            });
        });

        // Set up ICE candidate handler for answerer
        let p2p_manager_ice = Arc::clone(self);
        let peer_id_ice = from.to_string();
        let correlation_id_ice = correlation_id.to_string();
        transport.on_ice_candidate(move |candidate_json| {
            debug!("ICE candidate generated for answerer from peer {}: {}", peer_id_ice, candidate_json);
            let p2p = Arc::clone(&p2p_manager_ice);
            let peer = peer_id_ice.clone();
            let candidate = candidate_json.clone();
            let corr_id = correlation_id_ice.clone();
            tokio::spawn(async move {
                let signal = P2PSignal::IceCandidate {
                    from: p2p.session_id().unwrap_or_default(),
                    candidate,
                    correlation_id: corr_id,
                };
                if let Err(e) = p2p.send_signal(&peer, signal).await {
                    warn!("Failed to send ICE candidate: {}", e);
                }
            });
        });

        // Create connection entry
        let mut connection = P2PConnection::new(from.to_string(), correlation_id.to_string());
        connection.state = P2PConnectionState::GatheringCandidates;
        connection.transport = Some(transport);

        self.connections.insert(from.to_string(), connection);

        // Send answer
        let signal = P2PSignal::Answer {
            from: our_session_id,
            sdp: sdp_answer,
            correlation_id: correlation_id.to_string(),
        };

        self.send_signal(from, signal).await?;

        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    async fn handle_offer(&self, _from: &str, _sdp: &str, _correlation_id: &str) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "p2p")]
    async fn handle_answer(&self, from: &str, sdp: &str, correlation_id: &str) -> Result<()> {
        info!("Received P2P answer from {}", from);

        // First, check if we have a connection and extract what we need
        let (should_process, pending_candidates) = {
            if let Some(mut connection) = self.connections.get_mut(from) {
                if connection.correlation_id != correlation_id {
                    warn!("Correlation ID mismatch for answer from {}", from);
                    return Ok(());
                }

                if connection.transport.is_some() {
                    connection.state = P2PConnectionState::GatheringCandidates;
                    let pending = connection.take_pending_candidates();
                    (true, pending)
                } else {
                    (false, Vec::new())
                }
            } else {
                return Ok(());
            }
        };

        // Now process outside the borrow
        if should_process {
            if let Some(connection) = self.connections.get(from) {
                if let Some(ref transport) = connection.transport {
                    transport
                        .set_remote_answer(sdp)
                        .await
                        .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

                    // Process any pending ICE candidates
                    for candidate in pending_candidates {
                        if let Err(e) = transport.add_ice_candidate(&candidate).await {
                            warn!("Failed to add pending ICE candidate: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    async fn handle_answer(&self, _from: &str, _sdp: &str, _correlation_id: &str) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "p2p")]
    async fn handle_ice_candidate(
        &self,
        from: &str,
        candidate: &str,
        correlation_id: &str,
    ) -> Result<()> {
        debug!("Received ICE candidate from {}", from);

        if let Some(mut connection) = self.connections.get_mut(from) {
            if connection.correlation_id != correlation_id {
                return Ok(());
            }

            if let Some(ref transport) = connection.transport {
                // Try to add the candidate
                if let Err(e) = transport.add_ice_candidate(candidate).await {
                    // If remote description not set yet, queue the candidate
                    debug!("Queueing ICE candidate: {}", e);
                    connection.add_pending_candidate(candidate.to_string());
                }
            } else {
                // No transport yet, queue the candidate
                connection.add_pending_candidate(candidate.to_string());
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    async fn handle_ice_candidate(
        &self,
        _from: &str,
        _candidate: &str,
        _correlation_id: &str,
    ) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "p2p")]
    async fn handle_connected(&self, from: &str, correlation_id: &str) -> Result<()> {
        info!("P2P connected notification from {}", from);

        if let Some(mut connection) = self.connections.get_mut(from) {
            if connection.correlation_id == correlation_id {
                connection.state = P2PConnectionState::Connected;

                // Notify via callback
                if let Some(callback) = self.event_callback.read().as_ref() {
                    callback(P2PEvent::Connected {
                        peer_session_id: from.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    async fn handle_connected(&self, _from: &str, _correlation_id: &str) -> Result<()> {
        Ok(())
    }

    async fn handle_disconnected(
        &self,
        from: &str,
        _correlation_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        info!("P2P disconnected from {}: {:?}", from, reason);

        self.connections.remove(from);

        // Notify via callback
        if let Some(callback) = self.event_callback.read().as_ref() {
            callback(P2PEvent::Disconnected {
                peer_session_id: from.to_string(),
                reason: reason.map(|s| s.to_string()),
            });
        }

        Ok(())
    }

    /// Send a P2P signal to a peer via the router
    async fn send_signal(&self, target_session_id: &str, signal: P2PSignal) -> Result<()> {
        let address = signal_address(target_session_id);

        let payload =
            serde_json::to_value(&signal).map_err(|e| ClientError::Other(e.to_string()))?;

        let msg = Message::Publish(PublishMessage {
            address,
            signal: Some(SignalType::Event),
            value: None,
            payload: Some(value_from_json(payload)),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
            timeline: None,
        });

        self.signal_tx
            .send(msg)
            .await
            .map_err(|e| ClientError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Mark a peer connection as connected (called when DataChannel opens)
    #[cfg(feature = "p2p")]
    pub async fn mark_connected(&self, peer_session_id: &str) -> Result<()> {
        let our_session_id = self.session_id().ok_or(ClientError::NotConnected)?;

        if let Some(mut connection) = self.connections.get_mut(peer_session_id) {
            connection.state = P2PConnectionState::Connected;

            // Send connected notification to peer
            let signal = P2PSignal::Connected {
                from: our_session_id,
                correlation_id: connection.correlation_id.clone(),
            };

            drop(connection); // Release the lock before async operation
            self.send_signal(peer_session_id, signal).await?;

            // Notify via callback
            if let Some(callback) = self.event_callback.read().as_ref() {
                callback(P2PEvent::Connected {
                    peer_session_id: peer_session_id.to_string(),
                });
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "p2p"))]
    pub async fn mark_connected(&self, _peer_session_id: &str) -> Result<()> {
        Ok(())
    }
}

// =========================================================================
// Value conversion helpers
// =========================================================================

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::json!(*f),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
        Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Bytes(b) => {
            // Encode bytes as base64 string
            serde_json::Value::String(base64_encode(b))
        }
    }
}

fn value_from_json(json: serde_json::Value) -> Value {
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
            Value::Array(arr.into_iter().map(value_from_json).collect())
        }
        serde_json::Value::Object(obj) => {
            let map: std::collections::HashMap<String, Value> = obj
                .into_iter()
                .map(|(k, v)| (k, value_from_json(v)))
                .collect();
            Value::Map(map)
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversion() {
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "enabled": true,
            "tags": ["a", "b", "c"]
        });

        let value = value_from_json(json.clone());
        let back = value_to_json(&value);

        assert_eq!(json, back);
    }
}
