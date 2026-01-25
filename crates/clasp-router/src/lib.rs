//! # CLASP Router
//!
//! The router is the central message hub for CLASP (Creative Low-Latency Application Streaming Protocol).
//!
//! ## Core Responsibilities
//!
//! - **Session Management**: Track connected clients, handle authentication, manage session lifecycle
//! - **Message Routing**: Route messages between clients based on address patterns
//! - **State Management**: Maintain parameter state with revision tracking
//! - **Subscription Handling**: Match published messages to subscriber patterns
//! - **Protocol Bridging**: Interface with external protocols via bridges
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Client A  │     │   Client B  │     │   Client C  │
//! └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
//!        │                   │                   │
//!        └───────────────────┼───────────────────┘
//!                            │
//!                    ┌───────▼───────┐
//!                    │    Router     │
//!                    │  ┌─────────┐  │
//!                    │  │  State  │  │  Parameter storage
//!                    │  └─────────┘  │
//!                    │  ┌─────────┐  │
//!                    │  │Subscr.  │  │  Subscription matching
//!                    │  └─────────┘  │
//!                    │  ┌─────────┐  │
//!                    │  │Sessions │  │  Client tracking
//!                    │  └─────────┘  │
//!                    └───────────────┘
//! ```
//!
//! ## Transport Support
//!
//! The router is transport-agnostic and can accept connections from:
//!
//! - **WebSocket** (default): Universal, works in browsers and all platforms
//! - **QUIC**: High-performance for native apps (requires UDP)
//!
//! ## Quick Start
//!
//! ```no_run
//! use clasp_router::{Router, RouterConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create router with default configuration
//!     let router = Router::new(RouterConfig::default());
//!
//!     // Start WebSocket server on default port
//!     router.serve_websocket("0.0.0.0:7330").await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! ```no_run
//! use clasp_router::{Router, RouterConfig};
//!
//! let config = RouterConfig {
//!     max_sessions: 1000,
//!     max_subscriptions_per_session: 1000,
//!     session_timeout: 300,
//!     ..Default::default()
//! };
//!
//! let router = Router::new(config);
//! ```
//!
//! ## Message Flow
//!
//! 1. Client connects and sends HELLO
//! 2. Router responds with WELCOME (includes session ID, server time)
//! 3. Client subscribes to patterns
//! 4. Client sends SET/PUBLISH messages
//! 5. Router stores state (for SET) and routes to matching subscribers
//!
//! ## Module Overview
//!
//! - [`router`] - Main Router struct and message handling
//! - [`session`] - Client session management
//! - [`state`] - Parameter state storage
//! - [`subscription`] - Pattern-based subscription matching
//! - [`p2p`] - Peer-to-peer mesh networking support
//! - [`gesture`] - Gesture move coalescing for bandwidth optimization
//! - [`error`] - Error types

pub mod error;
pub mod gesture;
pub mod p2p;
pub mod router;
pub mod session;
pub mod state;
pub mod subscription;

// Protocol adapters (feature-gated)
#[cfg(any(feature = "mqtt-server", feature = "osc-server"))]
pub mod adapters;

pub use error::{Result, RouterError};
pub use gesture::{GestureRegistry, GestureResult};
pub use p2p::{analyze_address, P2PAddressType, P2PCapabilities};
pub use router::{MultiProtocolConfig, Router, RouterConfig, RouterConfigBuilder, TransportConfig};
#[cfg(feature = "quic")]
pub use router::QuicServerConfig;
pub use session::{Session, SessionId};
pub use state::RouterState;
pub use subscription::SubscriptionManager;

// Re-export adapter configs
#[cfg(feature = "mqtt-server")]
pub use adapters::{MqttServerAdapter, MqttServerConfig};
#[cfg(feature = "osc-server")]
pub use adapters::{OscServerAdapter, OscServerConfig};
