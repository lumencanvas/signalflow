//! CLASP Relay Server (Standalone)
//!
//! A minimal CLASP relay that uses published crates from crates.io.
//! This is the recommended way to deploy CLASP in production.
//!
//! # Usage
//!
//! ```bash
//! # Default (WebSocket on port 7330)
//! clasp-relay
//!
//! # Custom port
//! clasp-relay --port 8080
//!
//! # With verbose logging
//! clasp-relay --verbose
//! ```

use anyhow::Result;
use clap::Parser;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "clasp-relay")]
#[command(about = "CLASP Relay Server - routes messages between CLASP clients")]
#[command(version)]
struct Cli {
    /// Listen port
    #[arg(short, long, default_value = "7330")]
    port: u16,

    /// Listen host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Server name (shown in WELCOME)
    #[arg(short, long, default_value = "CLASP Relay")]
    name: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug,clasp=trace")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;

    tracing::info!("╔══════════════════════════════════════════════════════════════╗");
    tracing::info!("║              CLASP Relay Server                              ║");
    tracing::info!("╚══════════════════════════════════════════════════════════════╝");
    tracing::info!("Listening on: ws://{}", addr);
    tracing::info!("Server name: {}", cli.name);

    // Create router with open security (public relay)
    let config = RouterConfig {
        name: cli.name,
        security_mode: SecurityMode::Open,
        max_sessions: 1000,
        session_timeout: 300,
        features: vec![
            "param".to_string(),
            "event".to_string(),
            "stream".to_string(),
        ],
        max_subscriptions_per_session: 100,
        gesture_coalescing: false,
        gesture_coalesce_interval_ms: 16,
    };

    let router = Router::new(config);

    tracing::info!("Router initialized, accepting connections...");
    tracing::info!("Connect with: ws://{}:{}", cli.host, cli.port);

    // Serve WebSocket
    router.serve_websocket(&addr.to_string()).await?;

    Ok(())
}
