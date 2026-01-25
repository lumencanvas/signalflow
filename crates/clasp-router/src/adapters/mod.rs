//! Protocol Adapters for CLASP Router
//!
//! Adapters allow external protocol clients to connect directly to the CLASP router
//! without going through a separate broker. This enables CLASP to act as a multi-protocol
//! server accepting connections from various client types.
//!
//! ## Available Adapters
//!
//! - [`MqttServerAdapter`] - Accept MQTT clients on port 1883/8883
//! - [`OscServerAdapter`] - Accept OSC clients via UDP with session tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     CLASP Router                         │
//! │  ┌─────────────────────────────────────────────────────┐ │
//! │  │                   Shared State                       │ │
//! │  │  sessions | subscriptions | state | p2p_capabilities │ │
//! │  └─────────────────────────────────────────────────────┘ │
//! │        ▲               ▲               ▲                 │
//! │        │               │               │                 │
//! │  ┌─────┴─────┐   ┌─────┴─────┐   ┌─────┴─────┐          │
//! │  │ WebSocket │   │   MQTT    │   │    OSC    │          │
//! │  │  Server   │   │  Adapter  │   │  Adapter  │          │
//! │  └───────────┘   └───────────┘   └───────────┘          │
//! │    :7330           :1883           :8000                 │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! Adapters share the router's core state (sessions, subscriptions, state storage)
//! and translate between their native protocol and CLASP semantics.

pub mod mqtt_server;
pub mod osc_server;

pub use mqtt_server::{MqttServerAdapter, MqttServerConfig};
pub use osc_server::{OscServerAdapter, OscServerConfig};
