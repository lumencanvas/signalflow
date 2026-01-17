//! Main router implementation
//!
//! The router is transport-agnostic - it can accept connections from any transport
//! that implements the `TransportServer` trait (WebSocket, QUIC, TCP, etc.).
//!
//! # Transport Support
//!
//! - **WebSocket** (default): Works everywhere, including browsers and DO App Platform
//! - **QUIC**: High-performance for native apps. Requires UDP - NOT supported on DO App Platform
//! - **TCP**: Simple fallback, works everywhere
//!
//! # Example
//!
//! ```no_run
//! use clasp_router::{Router, RouterConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::new(RouterConfig::default());
//!
//!     // WebSocket (most common)
//!     router.serve_websocket("0.0.0.0:7330").await.unwrap();
//!
//!     // Or use any TransportServer implementation
//!     // router.serve_on(my_custom_server).await.unwrap();
//! }
//! ```

use bytes::Bytes;
use clasp_core::{
    codec, AckMessage, ErrorMessage, Frame, HelloMessage, Message, SetMessage, SignalType,
    SubscribeMessage, SubscribeOptions, UnsubscribeMessage, PROTOCOL_VERSION,
};
use clasp_transport::{TransportEvent, TransportReceiver, TransportSender, TransportServer};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[cfg(feature = "websocket")]
use clasp_transport::WebSocketServer;

#[cfg(feature = "quic")]
use clasp_transport::{QuicConfig, QuicTransport};

use crate::{
    error::{Result, RouterError},
    session::{Session, SessionId},
    state::RouterState,
    subscription::{Subscription, SubscriptionManager},
};

/// Transport configuration for multi-transport serving.
///
/// Use with `Router::serve_multi()` to run multiple transports simultaneously.
#[derive(Debug, Clone)]
pub enum TransportConfig {
    /// WebSocket transport (default, works everywhere)
    #[cfg(feature = "websocket")]
    WebSocket {
        /// Listen address, e.g., "0.0.0.0:7330"
        addr: String,
    },

    /// QUIC transport (high-performance, requires UDP)
    ///
    /// **WARNING**: Not supported on DigitalOcean App Platform or most PaaS.
    /// Use a VPS/Droplet for QUIC support.
    #[cfg(feature = "quic")]
    Quic {
        /// Listen address
        addr: SocketAddr,
        /// TLS certificate (DER format)
        cert: Vec<u8>,
        /// TLS private key (DER format)
        key: Vec<u8>,
    },
}

/// Router configuration
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Server name
    pub name: String,
    /// Supported features
    pub features: Vec<String>,
    /// Maximum sessions
    pub max_sessions: usize,
    /// Session timeout (seconds)
    pub session_timeout: u64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            name: "Clasp Router".to_string(),
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
                "timeline".to_string(),
            ],
            max_sessions: 100,
            session_timeout: 300,
        }
    }
}

/// Clasp router
pub struct Router {
    config: RouterConfig,
    /// Active sessions
    sessions: Arc<DashMap<SessionId, Arc<Session>>>,
    /// Subscription manager
    subscriptions: Arc<SubscriptionManager>,
    /// Global state
    state: Arc<RouterState>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl Router {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(DashMap::new()),
            subscriptions: Arc::new(SubscriptionManager::new()),
            state: Arc::new(RouterState::new()),
            running: Arc::new(RwLock::new(false)),
        }
    }

    // =========================================================================
    // Transport-Agnostic Methods
    // =========================================================================

    /// Serve using any TransportServer implementation.
    ///
    /// This is the core method that all transport-specific methods use internally.
    /// Use this when you have a custom transport or want full control.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use clasp_router::Router;
    /// use clasp_transport::WebSocketServer;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let router = Router::default();
    /// let server = WebSocketServer::bind("0.0.0.0:7330").await?;
    /// router.serve_on(server).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve_on<S>(&self, mut server: S) -> Result<()>
    where
        S: TransportServer + 'static,
        S::Sender: 'static,
        S::Receiver: 'static,
    {
        info!("Router accepting connections");
        *self.running.write() = true;

        while *self.running.read() {
            match server.accept().await {
                Ok((sender, receiver, addr)) => {
                    info!("New connection from {}", addr);
                    self.handle_connection(Arc::new(sender), receiver, addr);
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    // =========================================================================
    // WebSocket Transport
    // =========================================================================

    /// Start the router on WebSocket (default, recommended).
    ///
    /// WebSocket is the universal baseline transport:
    /// - Works in browsers
    /// - Works on all hosting platforms (including DO App Platform)
    /// - Easy firewall/proxy traversal
    ///
    /// Default port: 7330
    #[cfg(feature = "websocket")]
    pub async fn serve_websocket(&self, addr: &str) -> Result<()> {
        let server = WebSocketServer::bind(addr).await?;
        info!("WebSocket server listening on {}", addr);
        self.serve_on(server).await
    }

    /// Backward-compatible alias for `serve_websocket`.
    #[cfg(feature = "websocket")]
    pub async fn serve(&self, addr: &str) -> Result<()> {
        self.serve_websocket(addr).await
    }

    // =========================================================================
    // QUIC Transport (feature-gated)
    // =========================================================================

    /// Start the router on QUIC.
    ///
    /// QUIC is ideal for native applications:
    /// - 0-RTT connection establishment
    /// - Connection migration (mobile networks)
    /// - Built-in encryption (TLS 1.3)
    /// - Lower latency than WebSocket
    ///
    /// **WARNING**: QUIC requires UDP, which is NOT supported on:
    /// - DigitalOcean App Platform
    /// - Many PaaS providers
    /// - Some corporate firewalls
    ///
    /// Use a VPS/Droplet for QUIC support.
    ///
    /// Default port: 7331 (to avoid conflict with WebSocket on 7330)
    #[cfg(feature = "quic")]
    pub async fn serve_quic(
        &self,
        addr: SocketAddr,
        cert_der: Vec<u8>,
        key_der: Vec<u8>,
    ) -> Result<()> {
        let server = QuicTransport::new_server(addr, cert_der, key_der)
            .map_err(|e| RouterError::Transport(e))?;
        info!("QUIC server listening on {}", addr);
        self.serve_quic_transport(server).await
    }

    /// Internal: Serve using a QuicTransport server.
    ///
    /// QUIC has a different accept pattern (connection then stream),
    /// so we need special handling.
    #[cfg(feature = "quic")]
    async fn serve_quic_transport(&self, server: QuicTransport) -> Result<()> {
        *self.running.write() = true;

        while *self.running.read() {
            match server.accept().await {
                Ok(connection) => {
                    let addr = connection.remote_address();
                    info!("QUIC connection from {}", addr);

                    // Accept bidirectional stream for CLASP protocol
                    match connection.accept_bi().await {
                        Ok((sender, receiver)) => {
                            self.handle_connection(Arc::new(sender), receiver, addr);
                        }
                        Err(e) => {
                            error!("QUIC stream accept error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("QUIC accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    // =========================================================================
    // Multi-Transport Support
    // =========================================================================

    /// Serve on multiple transports simultaneously.
    ///
    /// All transports share the same router state, so a client connected via
    /// WebSocket can communicate with a client connected via QUIC.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use clasp_router::{Router, TransportConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let router = Router::default();
    /// router.serve_multi(vec![
    ///     TransportConfig::WebSocket { addr: "0.0.0.0:7330".into() },
    ///     // QUIC requires feature and UDP support
    ///     // TransportConfig::Quic { addr: "0.0.0.0:7331".parse()?, cert, key },
    /// ]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve_multi(&self, transports: Vec<TransportConfig>) -> Result<()> {
        use futures::future::try_join_all;

        if transports.is_empty() {
            return Err(RouterError::Config("No transports configured".into()));
        }

        let mut handles = vec![];

        for config in transports {
            let router = self.clone_internal();
            let handle = tokio::spawn(async move {
                match config {
                    #[cfg(feature = "websocket")]
                    TransportConfig::WebSocket { addr } => {
                        router.serve_websocket(&addr).await
                    }
                    #[cfg(feature = "quic")]
                    TransportConfig::Quic { addr, cert, key } => {
                        router.serve_quic(addr, cert, key).await
                    }
                    #[allow(unreachable_patterns)]
                    _ => Err(RouterError::Config("Transport not enabled at compile time".into())),
                }
            });
            handles.push(handle);
        }

        // Wait for all transports (or first error)
        let results = try_join_all(handles).await
            .map_err(|e| RouterError::Config(format!("Transport task failed: {}", e)))?;

        // Check for errors from any transport
        for result in results {
            result?;
        }

        Ok(())
    }

    /// Internal clone for spawning transport tasks.
    /// Shares all Arc state with the original.
    fn clone_internal(&self) -> Self {
        Self {
            config: self.config.clone(),
            sessions: Arc::clone(&self.sessions),
            subscriptions: Arc::clone(&self.subscriptions),
            state: Arc::clone(&self.state),
            running: Arc::clone(&self.running),
        }
    }

    /// Handle a new connection
    fn handle_connection(
        &self,
        sender: Arc<dyn TransportSender>,
        mut receiver: impl TransportReceiver + 'static,
        addr: SocketAddr,
    ) {
        let sessions = Arc::clone(&self.sessions);
        let subscriptions = Arc::clone(&self.subscriptions);
        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            let mut session: Option<Arc<Session>> = None;

            while *running.read() {
                match receiver.recv().await {
                    Some(TransportEvent::Data(data)) => {
                        // Decode message
                        match codec::decode(&data) {
                            Ok((msg, frame)) => {
                                // Handle message
                                if let Some(response) = handle_message(
                                    &msg,
                                    &frame,
                                    &session,
                                    &sender,
                                    &sessions,
                                    &subscriptions,
                                    &state,
                                    &config,
                                )
                                .await
                                {
                                    match response {
                                        MessageResult::NewSession(s) => {
                                            session = Some(s);
                                        }
                                        MessageResult::Send(bytes) => {
                                            if let Err(e) = sender.send(bytes).await {
                                                error!("Send error: {}", e);
                                                break;
                                            }
                                        }
                                        MessageResult::Broadcast(bytes, exclude) => {
                                            broadcast_to_subscribers(&bytes, &sessions, &exclude)
                                                .await;
                                        }
                                        MessageResult::None => {}
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Decode error from {}: {}", addr, e);
                            }
                        }
                    }
                    Some(TransportEvent::Disconnected { reason }) => {
                        info!("Client {} disconnected: {:?}", addr, reason);
                        break;
                    }
                    Some(TransportEvent::Error(e)) => {
                        error!("Transport error from {}: {}", addr, e);
                        break;
                    }
                    None => {
                        break;
                    }
                    _ => {}
                }
            }

            // Cleanup session
            if let Some(s) = session {
                info!("Removing session {}", s.id);
                sessions.remove(&s.id);
                subscriptions.remove_session(&s.id);
            }
        });
    }

    /// Stop the router
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get state
    pub fn state(&self) -> &RouterState {
        &self.state
    }

    /// Get subscription count
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new(RouterConfig::default())
    }
}

/// Result of handling a message
enum MessageResult {
    NewSession(Arc<Session>),
    Send(Bytes),
    Broadcast(Bytes, SessionId),
    None,
}

/// Handle an incoming message
async fn handle_message(
    msg: &Message,
    frame: &Frame,
    session: &Option<Arc<Session>>,
    sender: &Arc<dyn TransportSender>,
    sessions: &Arc<DashMap<SessionId, Arc<Session>>>,
    subscriptions: &Arc<SubscriptionManager>,
    state: &Arc<RouterState>,
    config: &RouterConfig,
) -> Option<MessageResult> {
    match msg {
        Message::Hello(hello) => {
            // Create new session
            let new_session = Arc::new(Session::new(
                sender.clone(),
                hello.name.clone(),
                hello.features.clone(),
            ));

            let session_id = new_session.id.clone();
            sessions.insert(session_id.clone(), new_session.clone());

            info!("Session created: {} ({})", hello.name, session_id);

            // Send welcome
            let welcome = new_session.welcome_message(&config.name, &config.features);
            let response = codec::encode(&welcome).ok()?;

            // Send welcome first
            let _ = sender.send(response).await;

            // Send initial snapshot
            let snapshot = Message::Snapshot(state.full_snapshot());
            let snapshot_bytes = codec::encode(&snapshot).ok()?;
            let _ = sender.send(snapshot_bytes).await;

            Some(MessageResult::NewSession(new_session))
        }

        Message::Subscribe(sub) => {
            let session = session.as_ref()?;

            // Create subscription
            match Subscription::new(
                sub.id,
                session.id.clone(),
                &sub.pattern,
                sub.types.clone(),
                sub.options.clone().unwrap_or_default(),
            ) {
                Ok(subscription) => {
                    subscriptions.add(subscription);
                    session.add_subscription(sub.id);

                    debug!("Session {} subscribed to {}", session.id, sub.pattern);

                    // Send matching current values
                    let snapshot = state.snapshot(&sub.pattern);
                    if !snapshot.params.is_empty() {
                        let msg = Message::Snapshot(snapshot);
                        let bytes = codec::encode(&msg).ok()?;
                        return Some(MessageResult::Send(bytes));
                    }
                }
                Err(e) => {
                    warn!("Invalid subscription pattern: {}", e);
                    let error = Message::Error(ErrorMessage {
                        code: 202,
                        message: e.to_string(),
                        address: Some(sub.pattern.clone()),
                        correlation_id: None,
                    });
                    let bytes = codec::encode(&error).ok()?;
                    return Some(MessageResult::Send(bytes));
                }
            }

            Some(MessageResult::None)
        }

        Message::Unsubscribe(unsub) => {
            let session = session.as_ref()?;
            subscriptions.remove(&session.id, unsub.id);
            session.remove_subscription(unsub.id);
            Some(MessageResult::None)
        }

        Message::Set(set) => {
            let session = session.as_ref()?;

            // Apply to state
            match state.apply_set(set, &session.id) {
                Ok(revision) => {
                    // Broadcast to subscribers
                    let subscribers =
                        subscriptions.find_subscribers(&set.address, Some(SignalType::Param));

                    // Create updated SET message with revision
                    let mut updated_set = set.clone();
                    updated_set.revision = Some(revision);
                    let broadcast_msg = Message::Set(updated_set);

                    if let Ok(bytes) = codec::encode(&broadcast_msg) {
                        // Send to all subscribers (including sender for confirmation)
                        for sub_session_id in subscribers {
                            if let Some(sub_session) = sessions.get(&sub_session_id) {
                                let _ = sub_session.send(bytes.clone()).await;
                            }
                        }
                    }

                    // Send ACK to sender
                    let ack = Message::Ack(AckMessage {
                        address: Some(set.address.clone()),
                        revision: Some(revision),
                        locked: None,
                        holder: None,
                        correlation_id: None,
                    });
                    let ack_bytes = codec::encode(&ack).ok()?;
                    return Some(MessageResult::Send(ack_bytes));
                }
                Err(e) => {
                    let error = Message::Error(ErrorMessage {
                        code: 400,
                        message: format!("{:?}", e),
                        address: Some(set.address.clone()),
                        correlation_id: None,
                    });
                    let bytes = codec::encode(&error).ok()?;
                    return Some(MessageResult::Send(bytes));
                }
            }
        }

        Message::Get(get) => {
            if let Some(param_state) = state.get_state(&get.address) {
                let snapshot = Message::Snapshot(clasp_core::SnapshotMessage {
                    params: vec![clasp_core::ParamValue {
                        address: get.address.clone(),
                        value: param_state.value,
                        revision: param_state.revision,
                        writer: Some(param_state.writer),
                        timestamp: Some(param_state.timestamp),
                    }],
                });
                let bytes = codec::encode(&snapshot).ok()?;
                return Some(MessageResult::Send(bytes));
            }

            Some(MessageResult::None)
        }

        Message::Publish(pub_msg) => {
            let session = session.as_ref()?;

            // Determine signal type
            let signal_type = pub_msg.signal;

            // Find subscribers
            let subscribers = subscriptions.find_subscribers(&pub_msg.address, signal_type);

            // Broadcast
            if let Ok(bytes) = codec::encode(msg) {
                for sub_session_id in subscribers {
                    if sub_session_id != session.id {
                        if let Some(sub_session) = sessions.get(&sub_session_id) {
                            let _ = sub_session.send(bytes.clone()).await;
                        }
                    }
                }
            }

            Some(MessageResult::None)
        }

        Message::Ping => {
            let pong = Message::Pong;
            let bytes = codec::encode(&pong).ok()?;
            Some(MessageResult::Send(bytes))
        }

        Message::Query(query) => {
            // Return signal definitions (simplified - would need schema registry)
            let result = Message::Result(clasp_core::ResultMessage { signals: vec![] });
            let bytes = codec::encode(&result).ok()?;
            Some(MessageResult::Send(bytes))
        }

        _ => Some(MessageResult::None),
    }
}

/// Broadcast to all sessions except one
async fn broadcast_to_subscribers(
    data: &Bytes,
    sessions: &Arc<DashMap<SessionId, Arc<Session>>>,
    exclude: &SessionId,
) {
    for entry in sessions.iter() {
        if entry.key() != exclude {
            let _ = entry.value().send(data.clone()).await;
        }
    }
}
