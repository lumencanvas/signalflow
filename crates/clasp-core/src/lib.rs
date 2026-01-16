//! CLASP Core
//!
//! Core types, encoding, and protocol primitives for CLASP v2.
//! Creative Low-Latency Application Streaming Protocol.
//!
//! This crate provides:
//! - Protocol message types ([`Message`], [`SignalType`])
//! - Binary frame encoding/decoding ([`Frame`], [`codec`])
//! - Address parsing and wildcard matching ([`Address`])
//! - State management primitives ([`ParamState`])
//! - Timing utilities ([`Timestamp`])

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod address;
pub mod codec;
pub mod error;
pub mod frame;
pub mod state;
pub mod time;
pub mod types;

pub use address::Address;
pub use codec::{decode, encode};
pub use error::{Error, Result};
pub use frame::Frame;
pub use state::ParamState;
pub use time::Timestamp;
pub use types::*;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 2;

/// Magic byte for frame identification
pub const MAGIC_BYTE: u8 = 0x53; // 'S' for Streaming

/// Default WebSocket port
pub const DEFAULT_WS_PORT: u16 = 7330;

/// Default UDP discovery port
pub const DEFAULT_DISCOVERY_PORT: u16 = 7331;

/// WebSocket subprotocol identifier
pub const WS_SUBPROTOCOL: &str = "clasp.v2";

/// mDNS service type
pub const MDNS_SERVICE_TYPE: &str = "_clasp._tcp.local.";
