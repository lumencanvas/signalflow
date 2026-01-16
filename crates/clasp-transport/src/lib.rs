//! SignalFlow Transport Layer
//!
//! This crate provides transport implementations for SignalFlow:
//! - WebSocket (primary, required)
//! - UDP (for LAN, low-latency)
//! - QUIC (optional, for modern native apps)
//! - Serial (optional, for hardware)

pub mod error;
pub mod traits;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "udp")]
pub mod udp;

#[cfg(feature = "quic")]
pub mod quic;

#[cfg(feature = "serial")]
pub mod serial;

pub use error::{TransportError, Result};
pub use traits::{Transport, TransportEvent, TransportSender, TransportReceiver, TransportServer};

#[cfg(feature = "websocket")]
pub use websocket::{WebSocketTransport, WebSocketConfig, WebSocketServer};

#[cfg(feature = "udp")]
pub use udp::{UdpTransport, UdpConfig};
