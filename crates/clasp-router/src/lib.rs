//! Clasp Router
//!
//! The router is the central hub for Clasp communication:
//! - Manages client sessions
//! - Routes messages between clients
//! - Maintains parameter state
//! - Handles subscriptions
//! - Bridges to other protocols
//!
//! # Transport Support
//!
//! The router is transport-agnostic and can accept connections from:
//! - **WebSocket** (default): Universal, works in browsers and all platforms
//! - **QUIC**: High-performance native apps (requires UDP, not available on DO App Platform)
//!
//! # Example
//!
//! ```no_run
//! use clasp_router::{Router, RouterConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let router = Router::new(RouterConfig::default());
//!
//!     // WebSocket on default port
//!     router.serve_websocket("0.0.0.0:7330").await?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod router;
pub mod session;
pub mod state;
pub mod subscription;

pub use error::{Result, RouterError};
pub use router::{Router, RouterConfig, TransportConfig};
pub use session::{Session, SessionId};
pub use state::RouterState;
pub use subscription::SubscriptionManager;
