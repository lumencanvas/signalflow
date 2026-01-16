//! CLASP Transport Layer
//!
//! This crate provides transport implementations for CLASP.
//! The protocol is transport-agnostic - any byte transport works.
//!
//! Available transports:
//! - WebSocket (recommended baseline for interoperability)
//! - UDP (LAN, low-latency, broadcast)
//! - QUIC (modern native apps, connection migration)
//! - Serial (direct hardware, lowest latency)
//! - BLE (Bluetooth Low Energy, wireless controllers)
//! - WebRTC (P2P, NAT traversal, low-latency)

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

#[cfg(feature = "ble")]
pub mod ble;

#[cfg(feature = "webrtc")]
pub mod webrtc;

pub use error::{Result, TransportError};
pub use traits::{Transport, TransportEvent, TransportReceiver, TransportSender, TransportServer};

#[cfg(feature = "websocket")]
pub use websocket::{WebSocketConfig, WebSocketServer, WebSocketTransport};

#[cfg(feature = "udp")]
pub use udp::{UdpConfig, UdpTransport};

#[cfg(feature = "ble")]
pub use ble::{BleConfig, BleTransport};

#[cfg(feature = "webrtc")]
pub use webrtc::{WebRtcConfig, WebRtcTransport};

#[cfg(feature = "quic")]
pub use quic::{QuicConfig, QuicConnection, QuicTransport};

#[cfg(feature = "serial")]
pub use serial::{SerialConfig, SerialTransport};
