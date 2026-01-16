//! Transport trait definitions

use async_trait::async_trait;
use bytes::Bytes;
use std::net::SocketAddr;

use crate::error::Result;

/// Events that can occur on a transport
#[derive(Debug, Clone)]
pub enum TransportEvent {
    /// Connection established
    Connected,
    /// Connection closed (clean or error)
    Disconnected { reason: Option<String> },
    /// Data received
    Data(Bytes),
    /// Error occurred
    Error(String),
}

/// Trait for sending data
#[async_trait]
pub trait TransportSender: Send + Sync {
    /// Send data
    async fn send(&self, data: Bytes) -> Result<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Close the sender
    async fn close(&self) -> Result<()>;
}

/// Trait for receiving data
#[async_trait]
pub trait TransportReceiver: Send {
    /// Receive the next event
    async fn recv(&mut self) -> Option<TransportEvent>;
}

/// Main transport trait
#[async_trait]
pub trait Transport: Send + Sync {
    /// The sender type for this transport
    type Sender: TransportSender;
    /// The receiver type for this transport
    type Receiver: TransportReceiver;

    /// Connect to a remote endpoint
    async fn connect(addr: &str) -> Result<(Self::Sender, Self::Receiver)>
    where
        Self: Sized;

    /// Get the local address (if applicable)
    fn local_addr(&self) -> Option<SocketAddr>;

    /// Get the remote address (if applicable)
    fn remote_addr(&self) -> Option<SocketAddr>;
}

/// Trait for transport servers (listeners)
#[async_trait]
pub trait TransportServer: Send + Sync {
    /// The sender type for accepted connections
    type Sender: TransportSender;
    /// The receiver type for accepted connections
    type Receiver: TransportReceiver;

    /// Accept a new connection
    async fn accept(&mut self) -> Result<(Self::Sender, Self::Receiver, SocketAddr)>;

    /// Get the local address
    fn local_addr(&self) -> Result<SocketAddr>;

    /// Close the server
    async fn close(&self) -> Result<()>;
}
