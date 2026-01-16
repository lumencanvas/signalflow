//! CLASP Router Server
//!
//! A standalone CLASP router that accepts connections and routes
//! messages between clients.

use anyhow::Result;
use clap::Parser;
use clasp_router::{router::RouterConfig, Router};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "bridges")]
use clasp_discovery::mdns::ServiceAdvertiser;

#[derive(Parser)]
#[command(name = "clasp-router")]
#[command(about = "CLASP Router Server")]
#[command(version)]
struct Cli {
    /// Listen address
    #[arg(short, long, default_value = "0.0.0.0:7330")]
    listen: SocketAddr,

    /// Server name for discovery
    #[arg(short, long, default_value = "CLASP Router")]
    name: String,

    /// Enable mDNS discovery announcement
    #[arg(short, long)]
    announce: bool,

    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("Starting CLASP Router");
    tracing::info!("Listening on: {}", cli.listen);

    // Start mDNS announcement if enabled
    #[cfg(feature = "bridges")]
    let _advertiser = if cli.announce {
        tracing::info!("Enabling mDNS discovery announcement");
        let mut advertiser = ServiceAdvertiser::new()?;
        advertiser.advertise(&cli.name, cli.listen.port(), &["param", "event", "stream"])?;
        Some(advertiser)
    } else {
        None
    };

    // Create router config
    let config = RouterConfig {
        name: cli.name.clone(),
        ..Default::default()
    };

    // Create and start router
    let router = Router::new(config);

    tracing::info!("Router ready, accepting connections...");

    // Run until interrupted
    let addr_str = cli.listen.to_string();
    router.serve(&addr_str).await?;

    Ok(())
}
