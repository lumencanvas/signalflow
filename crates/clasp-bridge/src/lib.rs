//! SignalFlow Protocol Bridges
//!
//! Provides bidirectional bridges between SignalFlow and legacy protocols:
//! - OSC (Open Sound Control)
//! - MIDI (Musical Instrument Digital Interface)
//! - Art-Net (Ethernet DMX)
//! - sACN/E1.31 (Streaming ACN)
//! - DMX-512 (via USB interfaces)

pub mod error;
pub mod traits;
pub mod mapping;

#[cfg(feature = "osc")]
pub mod osc;

#[cfg(feature = "midi")]
pub mod midi;

#[cfg(feature = "artnet")]
pub mod artnet;

#[cfg(feature = "dmx")]
pub mod dmx;

pub use error::{BridgeError, Result};
pub use traits::{Bridge, BridgeEvent, BridgeConfig};
pub use mapping::{AddressMapping, ValueTransform};

#[cfg(feature = "osc")]
pub use osc::{OscBridge, OscBridgeConfig};

#[cfg(feature = "midi")]
pub use midi::{MidiBridge, MidiBridgeConfig};

#[cfg(feature = "artnet")]
pub use artnet::{ArtNetBridge, ArtNetBridgeConfig};

#[cfg(feature = "dmx")]
pub use dmx::{DmxBridge, DmxBridgeConfig, DmxInterfaceType};
