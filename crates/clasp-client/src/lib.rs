//! SignalFlow Client Library
//!
//! High-level async client for SignalFlow protocol.
//!
//! # Example
//!
//! ```ignore
//! use clasp_client::SignalFlow;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let sf = SignalFlow::connect("wss://localhost:7330").await?;
//!
//!     // Subscribe to changes
//!     sf.subscribe("/lumen/scene/*/layer/*/opacity", |value, address| {
//!         println!("{} = {:?}", address, value);
//!     }).await?;
//!
//!     // Set a value
//!     sf.set("/lumen/scene/0/layer/0/opacity", 0.75).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod builder;
pub mod error;

pub use client::SignalFlow;
pub use builder::SignalFlowBuilder;
pub use error::{ClientError, Result};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::client::SignalFlow;
    pub use crate::builder::SignalFlowBuilder;
    pub use crate::error::{ClientError, Result};
    pub use clasp_core::{Message, Value, SignalType};
}
