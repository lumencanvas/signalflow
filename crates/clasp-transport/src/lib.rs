//! CLASP Transport Layer
//!
//! This crate provides transport implementations for CLASP.
//! The protocol is transport-agnostic - any byte transport works.
//!
//! Available transports:
//! - WebSocket (recommended baseline for interoperability)
//!   - Native: tokio-tungstenite (client + server)
//!   - WASM: web-sys (client only)
//! - UDP (LAN, low-latency, broadcast) - native only
//! - QUIC (modern native apps, connection migration) - native only
//! - Serial (direct hardware, lowest latency) - native only
//! - BLE (Bluetooth Low Energy, wireless controllers) - native only
//! - WebRTC (P2P, NAT traversal, low-latency)

pub mod error;
pub mod traits;

// Native WebSocket (uses tokio-tungstenite)
#[cfg(all(feature = "websocket", not(target_arch = "wasm32")))]
pub mod websocket;

// WASM WebSocket (uses web-sys)
#[cfg(all(feature = "wasm-websocket", target_arch = "wasm32"))]
pub mod wasm_websocket;

// Native-only transports
#[cfg(all(feature = "tcp", not(target_arch = "wasm32")))]
pub mod tcp;

#[cfg(all(feature = "udp", not(target_arch = "wasm32")))]
pub mod udp;

#[cfg(all(feature = "quic", not(target_arch = "wasm32")))]
pub mod quic;

#[cfg(all(feature = "serial", not(target_arch = "wasm32")))]
pub mod serial;

#[cfg(all(feature = "ble", not(target_arch = "wasm32")))]
pub mod ble;

#[cfg(feature = "webrtc")]
pub mod webrtc;

pub use error::{Result, TransportError};
pub use traits::{Transport, TransportEvent, TransportReceiver, TransportSender, TransportServer};

// Native WebSocket exports
#[cfg(all(feature = "websocket", not(target_arch = "wasm32")))]
pub use websocket::{WebSocketConfig, WebSocketServer, WebSocketTransport};

// WASM WebSocket exports
#[cfg(all(feature = "wasm-websocket", target_arch = "wasm32"))]
pub use wasm_websocket::{WasmWebSocketConfig, WasmWebSocketTransport};

#[cfg(all(feature = "tcp", not(target_arch = "wasm32")))]
pub use tcp::{TcpConfig, TcpServer, TcpTransport};

#[cfg(all(feature = "udp", not(target_arch = "wasm32")))]
pub use udp::{UdpConfig, UdpTransport};

#[cfg(all(feature = "ble", not(target_arch = "wasm32")))]
pub use ble::{BleConfig, BleTransport};

#[cfg(feature = "webrtc")]
pub use webrtc::{WebRtcConfig, WebRtcTransport};

#[cfg(all(feature = "quic", not(target_arch = "wasm32")))]
pub use quic::{QuicConfig, QuicConnection, QuicTransport};

#[cfg(all(feature = "serial", not(target_arch = "wasm32")))]
pub use serial::{SerialConfig, SerialTransport};
