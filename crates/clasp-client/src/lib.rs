//! Clasp Client Library
//!
//! High-level async client for Clasp protocol.
//!
//! # Example
//!
//! ```ignore
//! use clasp_client::Clasp;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let sf = Clasp::connect("wss://localhost:7330").await?;
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

pub use client::Clasp;
pub use builder::ClaspBuilder;
pub use error::{ClientError, Result};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::client::Clasp;
    pub use crate::builder::ClaspBuilder;
    pub use crate::error::{ClientError, Result};
    pub use clasp_core::{Message, Value, SignalType};
}
