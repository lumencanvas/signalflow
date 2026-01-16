//! Session management

use bytes::Bytes;
use parking_lot::RwLock;
use clasp_core::{Message, WelcomeMessage, PROTOCOL_VERSION};
use clasp_transport::TransportSender;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Session identifier
pub type SessionId = String;

/// A connected client session
pub struct Session {
    /// Unique session ID
    pub id: SessionId,
    /// Client name
    pub name: String,
    /// Client features
    pub features: Vec<String>,
    /// Transport sender for this session
    sender: Arc<dyn TransportSender>,
    /// Active subscriptions (subscription IDs)
    subscriptions: RwLock<HashSet<u32>>,
    /// Session creation time
    pub created_at: Instant,
    /// Last activity time
    pub last_activity: RwLock<Instant>,
    /// Is authenticated
    pub authenticated: bool,
    /// Permission token (if any)
    pub token: Option<String>,
}

impl Session {
    /// Create a new session
    pub fn new(sender: Arc<dyn TransportSender>, name: String, features: Vec<String>) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            features,
            sender,
            subscriptions: RwLock::new(HashSet::new()),
            created_at: now,
            last_activity: RwLock::new(now),
            authenticated: false,
            token: None,
        }
    }

    /// Send a message to this session
    pub async fn send(&self, data: Bytes) -> Result<(), clasp_transport::TransportError> {
        self.sender.send(data).await?;
        *self.last_activity.write() = Instant::now();
        Ok(())
    }

    /// Send a SignalFlow message
    pub async fn send_message(&self, message: &Message) -> Result<(), clasp_core::Error> {
        let data = clasp_core::codec::encode(message)?;
        self.send(data)
            .await
            .map_err(|e| clasp_core::Error::ConnectionError(e.to_string()))?;
        Ok(())
    }

    /// Create welcome message for this session
    pub fn welcome_message(&self, server_name: &str, server_features: &[String]) -> Message {
        Message::Welcome(WelcomeMessage {
            version: PROTOCOL_VERSION,
            session: self.id.clone(),
            name: server_name.to_string(),
            features: server_features.to_vec(),
            time: clasp_core::time::now(),
            token: None,
        })
    }

    /// Add a subscription
    pub fn add_subscription(&self, id: u32) {
        self.subscriptions.write().insert(id);
    }

    /// Remove a subscription
    pub fn remove_subscription(&self, id: u32) -> bool {
        self.subscriptions.write().remove(&id)
    }

    /// Get all subscription IDs
    pub fn subscriptions(&self) -> Vec<u32> {
        self.subscriptions.read().iter().cloned().collect()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.sender.is_connected()
    }

    /// Touch to update last activity
    pub fn touch(&self) {
        *self.last_activity.write() = Instant::now();
    }

    /// Get idle duration
    pub fn idle_duration(&self) -> std::time::Duration {
        self.last_activity.read().elapsed()
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("features", &self.features)
            .field("authenticated", &self.authenticated)
            .finish()
    }
}
