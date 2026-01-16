//! Bridge trait definitions

use async_trait::async_trait;
use clasp_core::Message;
use tokio::sync::mpsc;

use crate::Result;

/// Events from a bridge
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// Message to send to SignalFlow
    ToSignalFlow(Message),
    /// Bridge connected
    Connected,
    /// Bridge disconnected
    Disconnected { reason: Option<String> },
    /// Error occurred
    Error(String),
}

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Bridge name
    pub name: String,
    /// Protocol identifier
    pub protocol: String,
    /// Is bidirectional?
    pub bidirectional: bool,
    /// Protocol-specific options
    pub options: std::collections::HashMap<String, String>,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            name: "Bridge".to_string(),
            protocol: "unknown".to_string(),
            bidirectional: true,
            options: std::collections::HashMap::new(),
        }
    }
}

/// Main bridge trait
#[async_trait]
pub trait Bridge: Send + Sync {
    /// Get the bridge configuration
    fn config(&self) -> &BridgeConfig;

    /// Start the bridge
    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>>;

    /// Stop the bridge
    async fn stop(&mut self) -> Result<()>;

    /// Send a message from SignalFlow to the bridged protocol
    async fn send(&self, message: Message) -> Result<()>;

    /// Check if the bridge is running
    fn is_running(&self) -> bool;

    /// Get the namespace this bridge provides
    fn namespace(&self) -> &str;
}
