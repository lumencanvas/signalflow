//! Main router implementation

use bytes::Bytes;
use dashmap::DashMap;
use parking_lot::RwLock;
use clasp_core::{
    codec, AckMessage, ErrorMessage, Frame, HelloMessage, Message, SetMessage, SignalType,
    SubscribeMessage, SubscribeOptions, UnsubscribeMessage, PROTOCOL_VERSION,
};
use clasp_transport::{
    TransportEvent, TransportReceiver, TransportSender, TransportServer, WebSocketServer,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{
    error::{Result, RouterError},
    session::{Session, SessionId},
    state::RouterState,
    subscription::{Subscription, SubscriptionManager},
};

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
            name: "SignalFlow Router".to_string(),
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

/// SignalFlow router
pub struct Router {
    config: RouterConfig,
    /// Active sessions
    sessions: DashMap<SessionId, Arc<Session>>,
    /// Subscription manager
    subscriptions: SubscriptionManager,
    /// Global state
    state: RouterState,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl Router {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            sessions: DashMap::new(),
            subscriptions: SubscriptionManager::new(),
            state: RouterState::new(),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the router on WebSocket
    pub async fn serve(&self, addr: &str) -> Result<()> {
        let mut server = WebSocketServer::bind(addr).await?;

        info!("Router started on {}", addr);
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

    /// Handle a new connection
    fn handle_connection(
        &self,
        sender: Arc<dyn TransportSender>,
        mut receiver: impl TransportReceiver + 'static,
        addr: SocketAddr,
    ) {
        let sessions = self.sessions.clone();
        let subscriptions = self.subscriptions.clone();
        let state = self.state.clone();
        let config = self.config.clone();
        let running = self.running.clone();

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
                                            broadcast_to_subscribers(
                                                &bytes,
                                                &sessions,
                                                &exclude,
                                            )
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
    sessions: &DashMap<SessionId, Arc<Session>>,
    subscriptions: &SubscriptionManager,
    state: &RouterState,
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
                    let subscribers = subscriptions.find_subscribers(&set.address, Some(SignalType::Param));

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
    sessions: &DashMap<SessionId, Arc<Session>>,
    exclude: &SessionId,
) {
    for entry in sessions.iter() {
        if entry.key() != exclude {
            let _ = entry.value().send(data.clone()).await;
        }
    }
}

impl Clone for RouterState {
    fn clone(&self) -> Self {
        Self::new() // Fresh state - actual cloning would need proper implementation
    }
}

impl Clone for SubscriptionManager {
    fn clone(&self) -> Self {
        Self::new() // Fresh manager - actual cloning would need proper implementation
    }
}
