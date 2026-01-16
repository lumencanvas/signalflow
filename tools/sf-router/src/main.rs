//! SignalFlow Router Server
//!
//! A standalone SignalFlow router that accepts connections and routes
//! messages between clients.

use anyhow::Result;
use clap::Parser;
use clasp_discovery::Discovery;
use clasp_router::Router;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "sf-router")]
#[command(about = "SignalFlow Router Server")]
#[command(version)]
struct Cli {
    /// Listen address
    #[arg(short, long, default_value = "0.0.0.0:7330")]
    listen: SocketAddr,

    /// Server name for discovery
    #[arg(short, long, default_value = "SignalFlow Router")]
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

    tracing::info!("Starting SignalFlow Router");
    tracing::info!("Listening on: {}", cli.listen);

    // Start discovery if enabled
    let _discovery = if cli.announce {
        tracing::info!("Enabling mDNS discovery announcement");
        let discovery = Discovery::new().await?;
        discovery.announce(&cli.name, cli.listen.port()).await?;
        Some(discovery)
    } else {
        None
    };

    // Create and start router
    let router = Router::new(&cli.name);

    tracing::info!("Router ready, accepting connections...");

    // Run until interrupted
    router.serve(cli.listen).await?;

    Ok(())
}
