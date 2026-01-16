//! CLASP Protocol Bridges
//!
//! Provides bidirectional bridges between CLASP and various protocols:
//!
//! ## Legacy Creative Protocols
//! - OSC (Open Sound Control)
//! - MIDI (Musical Instrument Digital Interface)
//! - Art-Net (Ethernet DMX)
//! - sACN/E1.31 (Streaming ACN)
//! - DMX-512 (via USB interfaces)
//!
//! ## Modern Protocols
//! - MQTT (IoT messaging)
//! - WebSocket (real-time bidirectional)
//! - Socket.IO (event-based WebSocket)
//! - HTTP/REST (request-response API)

pub mod error;
pub mod traits;
pub mod mapping;
pub mod transform;

#[cfg(feature = "osc")]
pub mod osc;

#[cfg(feature = "midi")]
pub mod midi;

#[cfg(feature = "artnet")]
pub mod artnet;

#[cfg(feature = "dmx")]
pub mod dmx;

#[cfg(feature = "mqtt")]
pub mod mqtt;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "socketio")]
pub mod socketio;

#[cfg(feature = "http")]
pub mod http;

pub use error::{BridgeError, Result};
pub use traits::{Bridge, BridgeEvent, BridgeConfig};
pub use mapping::{AddressMapping, ValueTransform};
pub use transform::{Transform, TransformState, CurveType, Condition, Aggregator, AggregatorState};

#[cfg(feature = "osc")]
pub use osc::{OscBridge, OscBridgeConfig};

#[cfg(feature = "midi")]
pub use midi::{MidiBridge, MidiBridgeConfig};

#[cfg(feature = "artnet")]
pub use artnet::{ArtNetBridge, ArtNetBridgeConfig};

#[cfg(feature = "dmx")]
pub use dmx::{DmxBridge, DmxBridgeConfig, DmxInterfaceType};

#[cfg(feature = "mqtt")]
pub use mqtt::{MqttBridge, MqttBridgeConfig};

#[cfg(feature = "websocket")]
pub use websocket::{WebSocketBridge, WebSocketBridgeConfig, WsMode, WsMessageFormat};

#[cfg(feature = "socketio")]
pub use socketio::{SocketIOBridge, SocketIOBridgeConfig};

#[cfg(feature = "http")]
pub use http::{HttpBridge, HttpBridgeConfig, HttpMode, HttpMethod, EndpointConfig};
