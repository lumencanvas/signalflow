//! Main SignalFlow client implementation

use bytes::Bytes;
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use clasp_core::{
    codec, time::ClockSync, BundleMessage, GetMessage, HelloMessage, Message, PublishMessage,
    SetMessage, SignalType, SubscribeMessage, SubscribeOptions, UnsubscribeMessage, Value,
    PROTOCOL_VERSION,
};
use clasp_transport::{Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::builder::SignalFlowBuilder;
use crate::error::{ClientError, Result};

/// Subscription callback type
pub type SubscriptionCallback = Box<dyn Fn(Value, &str) + Send + Sync>;

/// A SignalFlow client
pub struct SignalFlow {
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
}

impl SignalFlow {
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
        }
    }

    /// Create a builder
    pub fn builder(url: &str) -> SignalFlowBuilder {
        SignalFlowBuilder::new(url)
    }

    /// Connect to server (convenience method)
    pub async fn connect_to(url: &str) -> Result<Self> {
        SignalFlowBuilder::new(url).connect().await
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
                    return Err(ClientError::ConnectionFailed("Connection closed".to_string()));
                }
                _ => {}
            }
        }

        // Spawn receiver task
        let params = Arc::clone(&self.params);
        let subscriptions = Arc::clone(&self.subscriptions);
        let pending_gets = Arc::clone(&self.pending_gets);
        let connected_clone = Arc::clone(&self.connected);

        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    TransportEvent::Data(data) => {
                        if let Ok((msg, _)) = codec::decode(&data) {
                            handle_message(&msg, &params, &subscriptions, &pending_gets);
                        }
                    }
                    TransportEvent::Disconnected { reason } => {
                        info!("Disconnected: {:?}", reason);
                        *connected_clone.write() = false;
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
        let sender = self.sender.read();
        if let Some(tx) = sender.as_ref() {
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

    /// Get current value (cached or request)
    pub async fn get(&self, address: &str) -> Result<Value> {
        // Check cache first
        if let Some(value) = self.params.get(address) {
            return Ok(value.clone());
        }

        // Request from server
        let (tx, rx) = oneshot::channel();
        self.pending_gets.insert(address.to_string(), tx);

        let msg = Message::Get(GetMessage {
            address: address.to_string(),
        });
        self.send_message(&msg).await?;

        // Wait for response (with timeout)
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => Err(ClientError::Other("Get cancelled".to_string())),
            Err(_) => Err(ClientError::Timeout),
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

    /// Close connection
    pub async fn close(&self) {
        *self.connected.write() = false;
        *self.sender.write() = None;
    }
}

/// Handle incoming message
fn handle_message(
    msg: &Message,
    params: &Arc<DashMap<String, Value>>,
    subscriptions: &Arc<DashMap<u32, (String, SubscriptionCallback)>>,
    pending_gets: &Arc<DashMap<String, oneshot::Sender<Value>>>,
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

        _ => {}
    }
}
