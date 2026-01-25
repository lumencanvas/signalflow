//! Session management

use bytes::Bytes;
use clasp_core::{Action, Message, Scope, WelcomeMessage, PROTOCOL_VERSION};
use clasp_transport::TransportSender;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
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
    /// Subject identifier from token (user, device, or service ID)
    pub subject: Option<String>,
    /// Scopes granted to this session
    scopes: Vec<Scope>,
    /// Messages received in the current second (for rate limiting)
    messages_this_second: AtomicU32,
    /// The second when the message count was last reset (Unix timestamp)
    last_rate_limit_second: AtomicU64,
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
            subject: None,
            scopes: Vec::new(),
            messages_this_second: AtomicU32::new(0),
            last_rate_limit_second: AtomicU64::new(0),
        }
    }

    /// Set authentication info from a validated token
    pub fn set_authenticated(
        &mut self,
        token: String,
        subject: Option<String>,
        scopes: Vec<Scope>,
    ) {
        self.authenticated = true;
        self.token = Some(token);
        self.subject = subject;
        self.scopes = scopes;
    }

    /// Check if this session has permission for the given action on the given address
    pub fn has_scope(&self, action: Action, address: &str) -> bool {
        // Unauthenticated sessions in open mode have no scope restrictions
        // (handled by router based on SecurityMode)
        if self.scopes.is_empty() && !self.authenticated {
            return true;
        }
        self.scopes
            .iter()
            .any(|scope| scope.allows(action, address))
    }

    /// Get the scopes for this session
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    /// Send a message to this session
    pub async fn send(&self, data: Bytes) -> Result<(), clasp_transport::TransportError> {
        self.sender.send(data).await?;
        *self.last_activity.write() = Instant::now();
        Ok(())
    }

    /// Try to send a message without blocking (for broadcasts)
    /// Returns Ok if sent or queued, Err if buffer is full
    pub fn try_send(&self, data: Bytes) -> Result<(), clasp_transport::TransportError> {
        self.sender.try_send(data)?;
        *self.last_activity.write() = Instant::now();
        Ok(())
    }

    /// Send a Clasp message
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

    /// Check and increment rate limit counter
    /// Returns true if within rate limit, false if exceeded
    pub fn check_rate_limit(&self, max_per_second: u32) -> bool {
        if max_per_second == 0 {
            return true; // No rate limiting
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let last_second = self.last_rate_limit_second.load(Ordering::Relaxed);

        if now != last_second {
            // New second, reset counter
            self.messages_this_second.store(1, Ordering::Relaxed);
            self.last_rate_limit_second.store(now, Ordering::Relaxed);
            true
        } else {
            // Same second, increment and check
            let count = self.messages_this_second.fetch_add(1, Ordering::Relaxed) + 1;
            count <= max_per_second
        }
    }

    /// Get current message count for this second
    pub fn messages_per_second(&self) -> u32 {
        self.messages_this_second.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("features", &self.features)
            .field("authenticated", &self.authenticated)
            .field("subject", &self.subject)
            .field("scopes", &self.scopes.len())
            .finish()
    }
}
