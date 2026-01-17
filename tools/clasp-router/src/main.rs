//! CLASP Router Server
//!
//! A standalone CLASP router that accepts connections and routes
//! messages between clients.
//!
//! # Transport Support
//!
//! - **WebSocket** (default): Works everywhere, including DigitalOcean App Platform
//! - **QUIC**: High-performance for native apps. Requires UDP - NOT supported on DO App Platform
//!
//! # Examples
//!
//! ```bash
//! # WebSocket on default port (works on DO App Platform)
//! clasp-router --listen 0.0.0.0:7330
//!
//! # QUIC with auto-generated self-signed cert (requires UDP, use on Droplet/VPS)
//! clasp-router --listen 0.0.0.0:7331 --transport quic
//!
//! # QUIC with custom certificate
//! clasp-router --transport quic --cert cert.der --key key.der
//! ```

use anyhow::Result;
use clap::{Parser, ValueEnum};
use clasp_router::{Router, RouterConfig};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "bridges")]
use clasp_discovery::mdns::ServiceAdvertiser;

/// Transport protocol to use
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Transport {
    /// WebSocket - universal, works in browsers and all platforms
    #[default]
    Websocket,

    /// QUIC - high-performance, requires UDP
    /// WARNING: Not supported on DigitalOcean App Platform (use Droplet instead)
    #[cfg(feature = "quic")]
    Quic,
}

#[derive(Parser)]
#[command(name = "clasp-router")]
#[command(about = "CLASP Router Server - routes messages between CLASP clients")]
#[command(version)]
#[command(after_help = r#"TRANSPORT NOTES:
  WebSocket (default): Works everywhere, including DO App Platform, browsers
  QUIC: High-performance for native apps. Requires UDP - use Droplet/VPS, NOT App Platform

EXAMPLES:
  # WebSocket server (default, works on DO App Platform)
  clasp-router --listen 0.0.0.0:7330

  # QUIC server with self-signed cert (requires UDP - Droplet/VPS only)
  clasp-router --listen 0.0.0.0:7331 --transport quic

  # With mDNS discovery announcement
  clasp-router --listen 0.0.0.0:7330 --announce --name "Studio Router"
"#)]
struct Cli {
    /// Listen address (host:port)
    #[arg(short, long, default_value = "0.0.0.0:7330")]
    listen: SocketAddr,

    /// Transport protocol
    #[arg(short, long, default_value = "websocket")]
    transport: Transport,

    /// Server name for discovery
    #[arg(short, long, default_value = "CLASP Router")]
    name: String,

    /// Enable mDNS discovery announcement
    #[arg(short, long)]
    announce: bool,

    /// TLS certificate file (DER format, for QUIC)
    #[arg(long)]
    cert: Option<String>,

    /// TLS private key file (DER format, for QUIC)
    #[arg(long)]
    key: Option<String>,

    /// Config file path (TOML)
    #[arg(short = 'C', long)]
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
    tracing::info!("Transport: {:?}", cli.transport);
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

    // Create router
    let router = Router::new(config);

    tracing::info!("Router ready, accepting connections...");

    // Run with appropriate transport
    match cli.transport {
        Transport::Websocket => {
            #[cfg(feature = "websocket")]
            {
                let addr_str = cli.listen.to_string();
                router.serve_websocket(&addr_str).await?;
            }
            #[cfg(not(feature = "websocket"))]
            {
                anyhow::bail!("WebSocket support not compiled in. Build with --features websocket");
            }
        }

        #[cfg(feature = "quic")]
        Transport::Quic => {
            // Load or generate TLS certificate
            let (cert_der, key_der) = if let (Some(cert_path), Some(key_path)) =
                (&cli.cert, &cli.key)
            {
                tracing::info!("Loading TLS certificate from files");
                let cert = std::fs::read(cert_path)?;
                let key = std::fs::read(key_path)?;
                (cert, key)
            } else {
                tracing::info!("Generating self-signed certificate for QUIC");
                generate_self_signed_cert()?
            };

            router.serve_quic(cli.listen, cert_der, key_der).await?;
        }
    }

    Ok(())
}

/// Generate a self-signed certificate for QUIC
#[cfg(feature = "quic")]
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>)> {
    use rcgen::{CertifiedKey, generate_simple_self_signed};

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    tracing::debug!("Generated self-signed certificate ({} bytes)", cert_der.len());
    Ok((cert_der, key_der))
}
