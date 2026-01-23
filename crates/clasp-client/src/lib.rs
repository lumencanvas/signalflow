//! # CLASP Client Library
//!
//! High-level async client for the CLASP (Creative Low-Latency Application Streaming Protocol).
//!
//! This crate provides a Rust client for connecting to CLASP routers, enabling real-time
//! communication between creative applications such as lighting controllers, audio mixers,
//! VJ software, and more.
//!
//! ## Features
//!
//! - **Async/await**: Built on Tokio for efficient async I/O
//! - **Builder pattern**: Flexible client configuration
//! - **Subscriptions**: Pattern-based subscriptions with callbacks
//! - **Parameters**: Get/set persistent values with caching
//! - **Events**: Fire-and-forget event emission
//! - **Streams**: High-rate data streaming (QoS fire)
//! - **Bundles**: Atomic multi-message operations
//! - **Time sync**: Automatic clock synchronization with server
//!
//! ## Quick Start
//!
//! ```ignore
//! use clasp_client::Clasp;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect using builder pattern
//!     let client = Clasp::builder("ws://localhost:7330")
//!         .name("my-app")
//!         .features(vec!["param", "event", "stream"])
//!         .connect()
//!         .await?;
//!
//!     println!("Connected! Session: {:?}", client.session_id());
//!
//!     // Subscribe to changes with wildcard patterns
//!     client.subscribe("/lumen/scene/*/layer/*/opacity", |value, address| {
//!         println!("{} = {:?}", address, value);
//!     }).await?;
//!
//!     // Set a parameter value
//!     client.set("/lumen/scene/0/layer/0/opacity", 0.75).await?;
//!
//!     // Emit an event (with map value)
//!     client.emit("/cue/trigger", clasp_core::Value::Map(Default::default())).await?;
//!
//!     // Stream high-rate data
//!     for i in 0..100 {
//!         let value = (i as f64 / 100.0).sin();
//!         client.stream("/sensor/value", value).await?;
//!         tokio::time::sleep(std::time::Duration::from_millis(10)).await;
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Address Patterns
//!
//! CLASP uses hierarchical addresses similar to OSC:
//!
//! - `/namespace/category/instance/property`
//! - `/lumen/scene/0/layer/3/opacity`
//! - `/midi/launchpad/cc/74`
//!
//! Wildcard patterns for subscriptions:
//!
//! - `*` - matches exactly one segment: `/path/*/value` matches `/path/foo/value`
//! - `**` - matches any number of segments: `/path/**` matches `/path/a/b/c`
//!
//! ## Signal Types
//!
//! | Type | Use Case | Persistence | QoS |
//! |------|----------|-------------|-----|
//! | `set()` | Parameters with state | Persisted | Confirm |
//! | `emit()` | One-shot events | Not persisted | Confirm |
//! | `stream()` | High-rate sensor data | Not persisted | Fire |
//! | `gesture()` | Touch/pen/motion input | Phase only | Fire |
//!
//! ## Error Handling
//!
//! All async methods return `Result<T, ClientError>`. Common errors:
//!
//! - `ClientError::NotConnected` - Operation requires active connection
//! - `ClientError::SendFailed` - Message could not be sent
//! - `ClientError::Timeout` - Operation timed out
//!
//! ## Crate Features
//!
//! - `p2p` - Enable peer-to-peer mesh networking support

pub mod builder;
pub mod client;
pub mod error;
#[cfg(feature = "p2p")]
pub mod p2p;

pub use builder::ClaspBuilder;
pub use client::Clasp;
pub use error::{ClientError, Result};
#[cfg(feature = "p2p")]
pub use p2p::{P2PEvent, P2PManager};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::builder::ClaspBuilder;
    pub use crate::client::Clasp;
    pub use crate::error::{ClientError, Result};
    #[cfg(feature = "p2p")]
    pub use crate::p2p::{P2PEvent, P2PManager};
    pub use clasp_core::{
        EasingType, GesturePhase, Message, SignalType, TimelineData, TimelineKeyframe, Value,
    };
}

// Re-export types for convenience
pub use clasp_core::{EasingType, GesturePhase, TimelineData, TimelineKeyframe};
