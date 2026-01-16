//! Clasp Router
//!
//! The router is the central hub for Clasp communication:
//! - Manages client sessions
//! - Routes messages between clients
//! - Maintains parameter state
//! - Handles subscriptions
//! - Bridges to other protocols

pub mod error;
pub mod router;
pub mod session;
pub mod state;
pub mod subscription;

pub use error::{Result, RouterError};
pub use router::Router;
pub use session::{Session, SessionId};
pub use state::RouterState;
pub use subscription::SubscriptionManager;
