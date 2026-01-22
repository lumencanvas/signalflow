//! CLASP Core
//!
//! Core types, encoding, and protocol primitives for CLASP v3.
//! Creative Low-Latency Application Streaming Protocol.
//!
//! # Wire Format v3
//!
//! CLASP v3 uses an efficient binary encoding that is ~54% smaller and ~5x faster
//! than the v2 MessagePack-with-named-keys format. The decoder auto-detects v2
//! format for backward compatibility.
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
#[cfg(feature = "std")]
pub mod p2p;
#[cfg(feature = "std")]
pub mod security;
pub mod state;
pub mod time;
pub mod types;

pub use address::Address;
pub use codec::{decode, encode};
pub use error::{Error, Result};
pub use frame::Frame;
#[cfg(feature = "std")]
pub use p2p::{
    extract_target_session, is_p2p_address, is_p2p_signal_address, signal_address, P2PAnnounce,
    P2PConfig, P2PConnectionState, P2PSignal, RoutingMode, TurnServer, P2P_ANNOUNCE, P2P_NAMESPACE,
    P2P_SIGNAL_PREFIX,
};
#[cfg(feature = "std")]
pub use security::{
    Action, CpskValidator, Scope, SecurityMode, TokenInfo, TokenValidator, ValidationResult,
    ValidatorChain,
};
pub use state::ParamState;
pub use time::Timestamp;
pub use types::*;

/// Protocol version (v3 = efficient binary encoding)
pub const PROTOCOL_VERSION: u8 = 3;

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
