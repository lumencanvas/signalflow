//! SignalFlow Router
//!
//! The router is the central hub for SignalFlow communication:
//! - Manages client sessions
//! - Routes messages between clients
//! - Maintains parameter state
//! - Handles subscriptions
//! - Bridges to other protocols

pub mod router;
pub mod session;
pub mod subscription;
pub mod state;
pub mod error;

pub use router::Router;
pub use session::{Session, SessionId};
pub use subscription::SubscriptionManager;
pub use state::RouterState;
pub use error::{RouterError, Result};
