//! Main Clasp client implementation

use bytes::Bytes;
use clasp_core::{
    codec, time::ClockSync, BundleMessage, ErrorMessage, GetMessage, GesturePhase, HelloMessage,
    Message, PublishMessage, SetMessage, SignalDefinition, SignalType, SubscribeMessage,
    SubscribeOptions, TimelineData, UnsubscribeMessage, Value, PROTOCOL_VERSION,
};
use clasp_transport::{
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Notify};
use tracing::{debug, error, info, warn};

use crate::builder::ClaspBuilder;
use crate::error::{ClientError, Result};
#[cfg(feature = "p2p")]
use crate::p2p;
#[cfg(feature = "p2p")]
use clasp_core::{P2PConfig, P2P_SIGNAL_PREFIX};

/// Subscription callback type
pub type SubscriptionCallback = Box<dyn Fn(Value, &str) + Send + Sync>;

/// A Clasp client
pub struct Clasp {
    url: String,
    name: String,
    features: Vec<String>,
    token: Option<String>,
    reconnect: bool,
    reconnect_interval_ms: u64,

    /// Session ID (set after connect)
    session_id: RwLock<Option<String>>,

    /// Connection state
    connected: Arc<RwLock<bool>>,

    /// Sender for outgoing messages
    sender: RwLock<Option<mpsc::Sender<Bytes>>>,

    /// Local param cache
    params: Arc<DashMap<String, Value>>,

    /// Subscriptions
    subscriptions: Arc<DashMap<u32, (String, SubscriptionCallback)>>,

    /// Subscription ID counter
    next_sub_id: AtomicU32,

    /// Clock synchronization
    clock: RwLock<ClockSync>,

    /// Pending get requests
    pending_gets: Arc<DashMap<String, oneshot::Sender<Value>>>,

    /// Announced signals (from server)
    signals: Arc<DashMap<String, SignalDefinition>>,

    /// Last error received from server
    last_error: Arc<RwLock<Option<ErrorMessage>>>,

    /// Reconnect attempt counter
    reconnect_attempts: Arc<AtomicU32>,

    /// Max reconnect attempts (0 = unlimited)
    max_reconnect_attempts: u32,

    /// Flag to indicate intentional close (don't reconnect)
    intentionally_closed: Arc<AtomicBool>,

    /// Notify for triggering reconnect
    reconnect_notify: Arc<Notify>,

    /// P2P config (optional, feature-gated)
    #[cfg(feature = "p2p")]
    p2p_config: Option<P2PConfig>,
    
    /// P2P manager (optional, feature-gated, created after connection)
    #[cfg(feature = "p2p")]
    p2p_manager: Option<Arc<p2p::P2PManager>>,
}

impl Clasp {
    /// Create a new client (use builder for more options)
    pub fn new(
        url: &str,
        name: String,
        features: Vec<String>,
        token: Option<String>,
        reconnect: bool,
        reconnect_interval_ms: u64,
    ) -> Self {
        Self {
            url: url.to_string(),
            name,
            features,
            token,
            reconnect,
            reconnect_interval_ms,
            session_id: RwLock::new(None),
            connected: Arc::new(RwLock::new(false)),
            sender: RwLock::new(None),
            params: Arc::new(DashMap::new()),
            subscriptions: Arc::new(DashMap::new()),
            next_sub_id: AtomicU32::new(1),
            clock: RwLock::new(ClockSync::new()),
            pending_gets: Arc::new(DashMap::new()),
            signals: Arc::new(DashMap::new()),
            last_error: Arc::new(RwLock::new(None)),
            reconnect_attempts: Arc::new(AtomicU32::new(0)),
            max_reconnect_attempts: 10,
            intentionally_closed: Arc::new(AtomicBool::new(false)),
            reconnect_notify: Arc::new(Notify::new()),
            #[cfg(feature = "p2p")]
            p2p_config: None,
            #[cfg(feature = "p2p")]
            p2p_manager: None,
        }
    }
    
    /// Set P2P configuration (internal, called by builder)
    #[cfg(feature = "p2p")]
    pub(crate) fn set_p2p_config(&mut self, config: P2PConfig) {
        self.p2p_config = Some(config);
    }

    /// Create a builder
    pub fn builder(url: &str) -> ClaspBuilder {
        ClaspBuilder::new(url)
    }

    /// Connect to server (convenience method)
    pub async fn connect_to(url: &str) -> Result<Self> {
        ClaspBuilder::new(url).connect().await
    }

    /// Internal connect
    pub(crate) async fn do_connect(&mut self) -> Result<()> {
        if *self.connected.read() {
            return Err(ClientError::AlreadyConnected);
        }

        info!("Connecting to {}", self.url);

        // Connect WebSocket
        let (sender, mut receiver) = <WebSocketTransport as Transport>::connect(&self.url).await?;

        // Create send channel
        let (tx, mut rx) = mpsc::channel::<Bytes>(100);
        *self.sender.write() = Some(tx);

        let connected = self.connected.clone();

        // Spawn sender task
        let sender = Arc::new(sender);
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                if let Err(e) = sender_clone.send(data).await {
                    error!("Send error: {}", e);
                    break;
                }
            }
        });

        // Send HELLO
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: self.name.clone(),
            features: self.features.clone(),
            capabilities: None,
            token: self.token.clone(),
        });

        self.send_message(&hello).await?;

        // Wait for WELCOME
        loop {
            match receiver.recv().await {
                Some(TransportEvent::Data(data)) => {
                    match codec::decode(&data) {
                        Ok((Message::Welcome(welcome), _)) => {
                            *self.session_id.write() = Some(welcome.session.clone());
                            *connected.write() = true;

                            // Sync clock
                            self.clock.write().process_sync(
                                clasp_core::time::now(),
                                welcome.time,
                                welcome.time,
                                clasp_core::time::now(),
                            );

                            // Initialize P2P manager if configured
                            #[cfg(feature = "p2p")]
                            {
                                if let Some(p2p_config) = self.p2p_config.take() {
                                    let session_id = welcome.session.clone();
                                    // Create channel for P2P signaling
                                    let (signal_tx, mut signal_rx) = mpsc::channel(100);
                                    let p2p_manager = Arc::new(p2p::P2PManager::new(p2p_config, signal_tx));
                                    p2p_manager.set_session_id(session_id.clone());
                                    
                                    // Spawn task to forward P2P signals through client
                                    let sender = self.sender.read().clone();
                                    let p2p_manager_for_task = Arc::clone(&p2p_manager);
                                    if let Some(sender_tx) = sender {
                                        tokio::spawn(async move {
                                            while let Some(msg) = signal_rx.recv().await {
                                                if let Some(encoded) = codec::encode(&msg).ok() {
                                                    if let Err(e) = sender_tx.send(encoded.into()).await {
                                                        tracing::warn!("Failed to send P2P signal: {}", e);
                                                        break;
                                                    }
                                                }
                                            }
                                        });
                                    }
                                    
                                    // Store P2P manager first
                                    self.p2p_manager = Some(Arc::clone(&p2p_manager));
                                    
                                    // Announce P2P capability
                                    let _ = p2p_manager.announce().await;
                                    
                                    // Set up P2P subscriptions (after manager is stored)
                                    let _ = self.setup_p2p_subscriptions(&session_id).await;
                                }
                            }

                            info!("Connected, session: {}", welcome.session);
                            break;
                        }
                        Ok((msg, _)) => {
                            debug!("Received during handshake: {:?}", msg);
                        }
                        Err(e) => {
                            warn!("Decode error: {}", e);
                        }
                    }
                }
                Some(TransportEvent::Error(e)) => {
                    return Err(ClientError::ConnectionFailed(e));
                }
                Some(TransportEvent::Disconnected { reason }) => {
                    return Err(ClientError::ConnectionFailed(
                        reason.unwrap_or_else(|| "Disconnected".to_string()),
                    ));
                }
                None => {
                    return Err(ClientError::ConnectionFailed(
                        "Connection closed".to_string(),
                    ));
                }
                _ => {}
            }
        }

        // Reset reconnect state on successful connect
        self.reconnect_attempts.store(0, Ordering::SeqCst);
        self.intentionally_closed.store(false, Ordering::SeqCst);

        // Spawn receiver task
        let params = Arc::clone(&self.params);
        let subscriptions = Arc::clone(&self.subscriptions);
        let pending_gets = Arc::clone(&self.pending_gets);
        let signals = Arc::clone(&self.signals);
        let last_error = Arc::clone(&self.last_error);
        let connected_clone = Arc::clone(&self.connected);
        let reconnect_notify = Arc::clone(&self.reconnect_notify);
        let intentionally_closed = Arc::clone(&self.intentionally_closed);
        let reconnect_enabled = self.reconnect;
        #[cfg(feature = "p2p")]
        let p2p_manager = self.p2p_manager.clone();

        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    TransportEvent::Data(data) => {
                        if let Ok((msg, _)) = codec::decode(&data) {
                            #[cfg(feature = "p2p")]
                            {
                                // Forward P2P signals to P2P manager (handled in subscription callback)
                                // P2P announce is also handled in subscription
                            }
                            handle_message(
                                &msg,
                                &params,
                                &subscriptions,
                                &pending_gets,
                                &signals,
                                &last_error,
                            );
                        }
                    }
                    TransportEvent::Disconnected { reason } => {
                        info!("Disconnected: {:?}", reason);
                        *connected_clone.write() = false;

                        // Trigger reconnect if enabled and not intentionally closed
                        if reconnect_enabled && !intentionally_closed.load(Ordering::SeqCst) {
                            reconnect_notify.notify_one();
                        }
                        break;
                    }
                    TransportEvent::Error(e) => {
                        error!("Error: {}", e);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Start the reconnect loop (call after initial connect)
    pub fn start_reconnect_loop(self: &Arc<Self>) {
        if !self.reconnect {
            return;
        }

        let client = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                // Wait for disconnect notification
                client.reconnect_notify.notified().await;

                if client.intentionally_closed.load(Ordering::SeqCst) {
                    break;
                }

                // Attempt reconnection with exponential backoff
                loop {
                    let attempts = client.reconnect_attempts.fetch_add(1, Ordering::SeqCst);

                    if client.max_reconnect_attempts > 0
                        && attempts >= client.max_reconnect_attempts
                    {
                        error!(
                            "Max reconnect attempts ({}) reached",
                            client.max_reconnect_attempts
                        );
                        break;
                    }

                    // Exponential backoff: base * 1.5^attempts, max 30 seconds
                    let base_ms = client.reconnect_interval_ms;
                    let delay_ms =
                        (base_ms as f64 * 1.5_f64.powi(attempts as i32)).min(30000.0) as u64;

                    info!("Reconnect attempt {} in {}ms", attempts + 1, delay_ms);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                    if client.intentionally_closed.load(Ordering::SeqCst) {
                        break;
                    }

                    // Clone the Arc and get a mutable reference for reconnection
                    // We need to use unsafe or restructure - let's use a different approach
                    match client.try_reconnect().await {
                        Ok(()) => {
                            info!("Reconnected successfully");
                            client.reconnect_attempts.store(0, Ordering::SeqCst);

                            // Resubscribe to all patterns
                            if let Err(e) = client.resubscribe_all().await {
                                warn!("Failed to resubscribe: {}", e);
                            }
                            break;
                        }
                        Err(e) => {
                            warn!("Reconnect failed: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Internal reconnect attempt
    async fn try_reconnect(&self) -> Result<()> {
        info!("Attempting to reconnect to {}", self.url);

        // Connect WebSocket
        let (sender, mut receiver) = <WebSocketTransport as Transport>::connect(&self.url).await?;

        // Create send channel
        let (tx, mut rx) = mpsc::channel::<Bytes>(100);
        *self.sender.write() = Some(tx);

        // Spawn sender task
        let sender = Arc::new(sender);
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                if let Err(e) = sender_clone.send(data).await {
                    error!("Send error: {}", e);
                    break;
                }
            }
        });

        // Send HELLO
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: self.name.clone(),
            features: self.features.clone(),
            capabilities: None,
            token: self.token.clone(),
        });

        self.send_message(&hello).await?;

        // Wait for WELCOME with timeout
        let welcome_timeout = Duration::from_secs(10);
        let deadline = tokio::time::Instant::now() + welcome_timeout;

        loop {
            match tokio::time::timeout_at(deadline, receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => match codec::decode(&data) {
                    Ok((Message::Welcome(welcome), _)) => {
                        *self.session_id.write() = Some(welcome.session.clone());
                        *self.connected.write() = true;

                        self.clock.write().process_sync(
                            clasp_core::time::now(),
                            welcome.time,
                            welcome.time,
                            clasp_core::time::now(),
                        );

                        info!("Reconnected, session: {}", welcome.session);
                        break;
                    }
                    Ok((msg, _)) => {
                        debug!("Received during reconnect handshake: {:?}", msg);
                    }
                    Err(e) => {
                        warn!("Decode error during reconnect: {}", e);
                    }
                },
                Ok(Some(TransportEvent::Error(e))) => {
                    return Err(ClientError::ConnectionFailed(e));
                }
                Ok(Some(TransportEvent::Disconnected { reason })) => {
                    return Err(ClientError::ConnectionFailed(
                        reason.unwrap_or_else(|| "Disconnected".to_string()),
                    ));
                }
                Ok(None) => {
                    return Err(ClientError::ConnectionFailed(
                        "Connection closed".to_string(),
                    ));
                }
                Err(_) => {
                    return Err(ClientError::Timeout);
                }
                _ => {}
            }
        }

        // Spawn new receiver task
        let params = Arc::clone(&self.params);
        let subscriptions = Arc::clone(&self.subscriptions);
        let pending_gets = Arc::clone(&self.pending_gets);
        let signals = Arc::clone(&self.signals);
        let last_error = Arc::clone(&self.last_error);
        let connected_clone = Arc::clone(&self.connected);
        let reconnect_notify = Arc::clone(&self.reconnect_notify);
        let intentionally_closed = Arc::clone(&self.intentionally_closed);
        let reconnect_enabled = self.reconnect;

        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    TransportEvent::Data(data) => {
                        if let Ok((msg, _)) = codec::decode(&data) {
                            handle_message(
                                &msg,
                                &params,
                                &subscriptions,
                                &pending_gets,
                                &signals,
                                &last_error,
                            );
                        }
                    }
                    TransportEvent::Disconnected { reason } => {
                        info!("Disconnected: {:?}", reason);
                        *connected_clone.write() = false;

                        if reconnect_enabled && !intentionally_closed.load(Ordering::SeqCst) {
                            reconnect_notify.notify_one();
                        }
                        break;
                    }
                    TransportEvent::Error(e) => {
                        error!("Error: {}", e);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Resubscribe to all existing subscriptions after reconnect
    async fn resubscribe_all(&self) -> Result<()> {
        // Collect subscription info first to avoid lifetime issues with DashMap
        let subs: Vec<(u32, String)> = self
            .subscriptions
            .iter()
            .map(|entry| (*entry.key(), entry.value().0.clone()))
            .collect();

        for (id, pattern) in subs {
            let msg = Message::Subscribe(SubscribeMessage {
                id,
                pattern: pattern.clone(),
                types: vec![],
                options: Some(SubscribeOptions::default()),
            });

            self.send_message(&msg).await?;
            debug!("Resubscribed to {} (id: {})", pattern, id);
        }

        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        *self.connected.read()
    }

    /// Get session ID
    pub fn session_id(&self) -> Option<String> {
        self.session_id.read().clone()
    }

    /// Get current server time (microseconds)
    pub fn time(&self) -> u64 {
        self.clock.read().server_time()
    }

    /// Send a raw message
    async fn send_message(&self, message: &Message) -> Result<()> {
        let data = codec::encode(message)?;
        self.send_raw(data).await
    }

    /// Send raw bytes
    async fn send_raw(&self, data: Bytes) -> Result<()> {
        // Clone the sender to avoid holding the lock across await
        let tx = {
            let sender = self.sender.read();
            sender.as_ref().cloned()
        };

        if let Some(tx) = tx {
            tx.send(data)
                .await
                .map_err(|e| ClientError::SendFailed(e.to_string()))?;
            Ok(())
        } else {
            Err(ClientError::NotConnected)
        }
    }

    /// Subscribe to an address pattern
    pub async fn subscribe<F>(&self, pattern: &str, callback: F) -> Result<u32>
    where
        F: Fn(Value, &str) + Send + Sync + 'static,
    {
        let id = self.next_sub_id.fetch_add(1, Ordering::SeqCst);

        // Store callback
        self.subscriptions
            .insert(id, (pattern.to_string(), Box::new(callback)));

        // Send subscribe message
        let msg = Message::Subscribe(SubscribeMessage {
            id,
            pattern: pattern.to_string(),
            types: vec![],
            options: Some(SubscribeOptions::default()),
        });

        self.send_message(&msg).await?;

        debug!("Subscribed to {} (id: {})", pattern, id);
        Ok(id)
    }

    /// Shorthand for subscribe
    pub async fn on<F>(&self, pattern: &str, callback: F) -> Result<u32>
    where
        F: Fn(Value, &str) + Send + Sync + 'static,
    {
        self.subscribe(pattern, callback).await
    }

    /// Unsubscribe
    pub async fn unsubscribe(&self, id: u32) -> Result<()> {
        self.subscriptions.remove(&id);

        let msg = Message::Unsubscribe(UnsubscribeMessage { id });
        self.send_message(&msg).await?;

        Ok(())
    }

    /// Set a parameter value
    pub async fn set(&self, address: &str, value: impl Into<Value>) -> Result<()> {
        let msg = Message::Set(SetMessage {
            address: address.to_string(),
            value: value.into(),
            revision: None,
            lock: false,
            unlock: false,
        });

        self.send_message(&msg).await
    }

    /// Set with lock
    pub async fn set_locked(&self, address: &str, value: impl Into<Value>) -> Result<()> {
        let msg = Message::Set(SetMessage {
            address: address.to_string(),
            value: value.into(),
            revision: None,
            lock: true,
            unlock: false,
        });

        self.send_message(&msg).await
    }

    /// Set and unlock (release a previously held lock)
    pub async fn set_unlocked(&self, address: &str, value: impl Into<Value>) -> Result<()> {
        let msg = Message::Set(SetMessage {
            address: address.to_string(),
            value: value.into(),
            revision: None,
            lock: false,
            unlock: true,
        });

        self.send_message(&msg).await
    }

    /// Get current value (cached or request)
    pub async fn get(&self, address: &str) -> Result<Value> {
        // Check cache first
        if let Some(value) = self.params.get(address) {
            return Ok(value.clone());
        }

        // Request from server
        let (tx, rx) = oneshot::channel();
        let address_key = address.to_string();
        self.pending_gets.insert(address_key.clone(), tx);

        let msg = Message::Get(GetMessage {
            address: address.to_string(),
        });
        self.send_message(&msg).await?;

        // Wait for response (with timeout)
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => {
                // Cancelled - remove from pending
                self.pending_gets.remove(&address_key);
                Err(ClientError::Other("Get cancelled".to_string()))
            }
            Err(_) => {
                // Timeout - remove from pending to prevent memory leak
                self.pending_gets.remove(&address_key);
                Err(ClientError::Timeout)
            }
        }
    }

    /// Emit an event
    pub async fn emit(&self, address: &str, payload: impl Into<Value>) -> Result<()> {
        let msg = Message::Publish(PublishMessage {
            address: address.to_string(),
            signal: Some(SignalType::Event),
            value: None,
            payload: Some(payload.into()),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: Some(self.time()),
            timeline: None,
        });

        self.send_message(&msg).await
    }

    /// Send stream sample
    pub async fn stream(&self, address: &str, value: impl Into<Value>) -> Result<()> {
        let msg = Message::Publish(PublishMessage {
            address: address.to_string(),
            signal: Some(SignalType::Stream),
            value: Some(value.into()),
            payload: None,
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: Some(self.time()),
            timeline: None,
        });

        self.send_message(&msg).await
    }

    /// Send gesture input
    ///
    /// Gestures are phased input streams for touch/pen/motion input.
    /// Each gesture has a stable ID and goes through phases:
    /// - `Start`: Begin a new gesture
    /// - `Move`: Update position/state (may be coalesced by router)
    /// - `End`: Complete the gesture normally
    /// - `Cancel`: Abort the gesture
    ///
    /// # Example
    /// ```ignore
    /// // Start a touch gesture
    /// client.gesture("/input/touch", 1, GesturePhase::Start, json!({"x": 0.5, "y": 0.3})).await?;
    ///
    /// // Move updates
    /// client.gesture("/input/touch", 1, GesturePhase::Move, json!({"x": 0.6, "y": 0.4})).await?;
    ///
    /// // End the gesture
    /// client.gesture("/input/touch", 1, GesturePhase::End, json!({"x": 0.7, "y": 0.5})).await?;
    /// ```
    pub async fn gesture(
        &self,
        address: &str,
        id: u32,
        phase: GesturePhase,
        payload: impl Into<Value>,
    ) -> Result<()> {
        let msg = Message::Publish(PublishMessage {
            address: address.to_string(),
            signal: Some(SignalType::Gesture),
            value: None,
            payload: Some(payload.into()),
            samples: None,
            rate: None,
            id: Some(id),
            phase: Some(phase),
            timestamp: Some(self.time()),
            timeline: None,
        });

        self.send_message(&msg).await
    }

    /// Publish timeline automation
    ///
    /// Timelines are pre-computed automation curves with keyframes.
    /// Once published, timelines are immutable - to modify, publish a new one.
    ///
    /// # Arguments
    /// * `address` - The parameter address this timeline controls
    /// * `timeline` - The timeline data with keyframes
    ///
    /// # Example
    /// ```ignore
    /// use clasp_core::{TimelineData, TimelineKeyframe, EasingType, Value};
    ///
    /// let timeline = TimelineData::new(vec![
    ///     TimelineKeyframe { time: 0, value: Value::Float(0.0), easing: EasingType::Linear, bezier: None },
    ///     TimelineKeyframe { time: 1_000_000, value: Value::Float(1.0), easing: EasingType::EaseOut, bezier: None },
    /// ]);
    ///
    /// client.timeline("/lights/master/dimmer", timeline).await?;
    /// ```
    pub async fn timeline(&self, address: &str, timeline_data: TimelineData) -> Result<()> {
        let msg = Message::Publish(PublishMessage {
            address: address.to_string(),
            signal: Some(SignalType::Timeline),
            value: None,
            payload: None,
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: Some(self.time()),
            timeline: Some(timeline_data),
        });

        self.send_message(&msg).await
    }

    /// Send atomic bundle
    pub async fn bundle(&self, messages: Vec<Message>) -> Result<()> {
        let msg = Message::Bundle(BundleMessage {
            timestamp: None,
            messages,
        });

        self.send_message(&msg).await
    }

    /// Send scheduled bundle
    pub async fn bundle_at(&self, messages: Vec<Message>, time: u64) -> Result<()> {
        let msg = Message::Bundle(BundleMessage {
            timestamp: Some(time),
            messages,
        });

        self.send_message(&msg).await
    }

    /// Get cached param value
    pub fn cached(&self, address: &str) -> Option<Value> {
        self.params.get(address).map(|v| v.clone())
    }

    /// Close connection.
    /// Disables auto-reconnect and closes the connection.
    pub async fn close(&self) {
        self.intentionally_closed.store(true, Ordering::SeqCst);
        *self.connected.write() = false;
        *self.sender.write() = None;
    }

    /// Get all announced signals
    pub fn signals(&self) -> Vec<SignalDefinition> {
        self.signals.iter().map(|e| e.value().clone()).collect()
    }

    /// Query signals matching a pattern
    pub fn query_signals(&self, pattern: &str) -> Vec<SignalDefinition> {
        self.signals
            .iter()
            .filter(|e| clasp_core::address::glob_match(pattern, e.key()))
            .map(|e| e.value().clone())
            .collect()
    }

    /// Get the last error received from server
    pub fn last_error(&self) -> Option<ErrorMessage> {
        self.last_error.read().clone()
    }

    /// Clear the last error
    pub fn clear_error(&self) {
        *self.last_error.write() = None;
    }

    /// Set up P2P subscriptions (internal, called after P2P manager is created)
    #[cfg(feature = "p2p")]
    async fn setup_p2p_subscriptions(&self, session_id: &str) -> Result<()> {
        if let Some(ref p2p_manager) = self.p2p_manager {
            let signal_address = format!("{}{}", P2P_SIGNAL_PREFIX, session_id);
            let p2p_manager_signal = Arc::clone(p2p_manager);
            
            // Subscribe to P2P signals
            let _ = self.subscribe(&signal_address, move |value, address| {
                let p2p = Arc::clone(&p2p_manager_signal);
                let address = address.to_string(); // Clone the address string
                tokio::spawn(async move {
                    if let Err(e) = p2p.handle_signal(&address, &value).await {
                        tracing::debug!("P2P signal handling error: {}", e);
                    }
                });
            }).await?;
            
            // Subscribe to P2P announce
            let p2p_manager_announce = Arc::clone(p2p_manager);
            let _ = self.subscribe(clasp_core::P2P_ANNOUNCE, move |value, _| {
                p2p_manager_announce.handle_announce(&value);
            }).await?;
        }
        Ok(())
    }

    /// Connect to a peer via P2P (requires p2p feature)
    #[cfg(feature = "p2p")]
    pub async fn connect_to_peer(&self, peer_session_id: &str) -> Result<()> {
        if let Some(ref p2p_manager) = self.p2p_manager {
            p2p_manager.connect_to_peer(peer_session_id).await
        } else {
            Err(ClientError::Other("P2P not configured. Use builder.p2p_config() to enable.".to_string()))
        }
    }

    /// Set P2P event callback (requires p2p feature)
    #[cfg(feature = "p2p")]
    pub fn on_p2p_event<F>(&self, callback: F)
    where
        F: Fn(p2p::P2PEvent) + Send + Sync + 'static,
    {
        if let Some(ref p2p_manager) = self.p2p_manager {
            p2p_manager.on_event(callback);
        }
    }

    /// Check if peer is connected via P2P (requires p2p feature)
    #[cfg(feature = "p2p")]
    pub fn is_peer_connected(&self, peer_session_id: &str) -> bool {
        self.p2p_manager
            .as_ref()
            .map(|p2p| p2p.is_peer_connected(peer_session_id))
            .unwrap_or(false)
    }
}

/// Handle incoming message
fn handle_message(
    msg: &Message,
    params: &Arc<DashMap<String, Value>>,
    subscriptions: &Arc<DashMap<u32, (String, SubscriptionCallback)>>,
    pending_gets: &Arc<DashMap<String, oneshot::Sender<Value>>>,
    signals: &Arc<DashMap<String, SignalDefinition>>,
    last_error: &Arc<RwLock<Option<ErrorMessage>>>,
) {
    match msg {
        Message::Set(set) => {
            // Update cache
            params.insert(set.address.clone(), set.value.clone());

            // Notify subscribers
            for entry in subscriptions.iter() {
                let (pattern, callback) = entry.value();
                if clasp_core::address::glob_match(pattern, &set.address) {
                    callback(set.value.clone(), &set.address);
                }
            }
        }

        Message::Snapshot(snapshot) => {
            for param in &snapshot.params {
                params.insert(param.address.clone(), param.value.clone());

                // Complete pending gets
                if let Some((_, tx)) = pending_gets.remove(&param.address) {
                    let _ = tx.send(param.value.clone());
                }

                // Notify subscribers
                for entry in subscriptions.iter() {
                    let (pattern, callback) = entry.value();
                    if clasp_core::address::glob_match(pattern, &param.address) {
                        callback(param.value.clone(), &param.address);
                    }
                }
            }
        }

        Message::Publish(pub_msg) => {
            #[cfg(feature = "p2p")]
            {
                // Check if this is a P2P signal or announce - handle before regular subscriptions
                if pub_msg.address.starts_with(clasp_core::P2P_SIGNAL_PREFIX) {
                    // P2P signals will be handled by subscription callbacks
                    // (they're subscribed to automatically when P2P manager is created)
                } else if pub_msg.address == clasp_core::P2P_ANNOUNCE {
                    // P2P announce will be handled by subscription callback
                }
            }
            
            // Notify subscribers
            let value = pub_msg
                .value
                .clone()
                .or_else(|| pub_msg.payload.clone())
                .unwrap_or(Value::Null);

            for entry in subscriptions.iter() {
                let (pattern, callback) = entry.value();
                if clasp_core::address::glob_match(pattern, &pub_msg.address) {
                    callback(value.clone(), &pub_msg.address);
                }
            }
        }

        Message::Error(error) => {
            // Log the error and store it for retrieval
            warn!(
                "Server error {}: {} (address: {:?})",
                error.code, error.message, error.address
            );
            *last_error.write() = Some(error.clone());
        }

        Message::Ack(ack) => {
            // Log acknowledgment (could be extended to track pending requests)
            debug!(
                "Received ACK for {:?} (revision: {:?})",
                ack.address, ack.revision
            );
        }

        Message::Announce(announce) => {
            // Store announced signals
            for signal in &announce.signals {
                debug!("Received signal announcement: {}", signal.address);
                signals.insert(signal.address.clone(), signal.clone());
            }
        }

        Message::Sync(sync) => {
            // Process clock sync response
            // Note: This handles sync messages from server with t2/t3 filled in
            if let (Some(t2), Some(t3)) = (sync.t2, sync.t3) {
                debug!("Clock sync: t1={}, t2={}, t3={}", sync.t1, t2, t3);
                // Clock sync is processed through ClockSync::process_sync
                // but we don't have access to the clock here.
                // For now, log it. A more complete implementation would
                // use a channel to send sync data back to the main client.
            }
        }

        Message::Result(result) => {
            // Handle query results
            debug!("Received result with {} signals", result.signals.len());
            // Store any returned signals
            for signal in &result.signals {
                signals.insert(signal.address.clone(), signal.clone());
            }
        }

        // Messages that are typically client-initiated, not expected from server
        Message::Hello(_)
        | Message::Welcome(_)
        | Message::Subscribe(_)
        | Message::Unsubscribe(_)
        | Message::Get(_)
        | Message::Query(_) => {
            debug!("Received unexpected client-type message: {:?}", msg);
        }

        // Bundle: process contained messages recursively
        Message::Bundle(bundle) => {
            for inner_msg in &bundle.messages {
                handle_message(
                    inner_msg,
                    params,
                    subscriptions,
                    pending_gets,
                    signals,
                    last_error,
                );
            }
        }

        // Ping/Pong for keep-alive
        Message::Ping => {
            debug!("Received PING from server");
            // Note: Pong response should be sent, but we don't have sender access here.
            // A more complete implementation would use a channel to request pong be sent.
        }

        Message::Pong => {
            debug!("Received PONG from server");
        }
    }
}
