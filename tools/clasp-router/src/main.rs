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
use clasp_core::{CpskValidator, Scope, SecurityMode, TokenInfo};
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

/// Security/authentication mode
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum AuthMode {
    /// Open - no authentication required (default)
    #[default]
    Open,

    /// Authenticated - require valid tokens for all connections
    Authenticated,
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

    /// Security/authentication mode
    #[arg(long, default_value = "open")]
    auth_mode: AuthMode,

    /// Token file for authenticated mode (one CPSK token per line)
    /// Format: cpsk_<base62-random-32-chars>
    #[arg(long)]
    token_file: Option<String>,

    /// Single token for authenticated mode (alternative to --token-file)
    #[arg(long)]
    token: Option<String>,

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

    // Determine security mode
    let security_mode = match cli.auth_mode {
        AuthMode::Open => SecurityMode::Open,
        AuthMode::Authenticated => SecurityMode::Authenticated,
    };

    // Create router config
    let config = RouterConfig {
        name: cli.name.clone(),
        security_mode,
        ..Default::default()
    };

    // Create router with optional token validator
    let router = if security_mode == SecurityMode::Authenticated {
        // Load tokens from file or CLI argument
        let validator = CpskValidator::new();

        // Helper to register a token with scope strings
        let register_token = |token: &str, scope_strs: Vec<&str>| -> Result<()> {
            let scopes: Vec<Scope> = scope_strs
                .iter()
                .map(|s| Scope::parse(s).map_err(|e| anyhow::anyhow!("Invalid scope '{}': {}", s, e)))
                .collect::<Result<Vec<_>>>()?;
            let info = TokenInfo::new(token.to_string(), scopes);
            validator.register(token.to_string(), info);
            Ok(())
        };

        // Add token from CLI argument
        if let Some(token) = &cli.token {
            tracing::info!("Adding token from CLI argument");
            // Parse scopes from token (format: token or token scope1,scope2)
            if token.contains(' ') {
                let parts: Vec<&str> = token.splitn(2, ' ').collect();
                let scopes: Vec<&str> = parts[1].split(',').map(|s| s.trim()).collect();
                register_token(parts[0], scopes)?;
            } else {
                // Default to full admin access
                register_token(token, vec!["admin:/**"])?;
            }
        }

        // Load tokens from file
        if let Some(token_file) = &cli.token_file {
            tracing::info!("Loading tokens from file: {}", token_file);
            let contents = std::fs::read_to_string(token_file)?;
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // Format: token or token scope1,scope2 (space-separated)
                if line.contains(' ') {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    let scopes: Vec<&str> = parts[1].split(',').map(|s| s.trim()).collect();
                    register_token(parts[0], scopes)?;
                } else {
                    // Token only - default to admin access
                    register_token(line, vec!["admin:/**"])?;
                }
            }
        }

        if validator.is_empty() {
            anyhow::bail!("Authenticated mode requires at least one token (use --token or --token-file)");
        }

        tracing::info!("Security mode: Authenticated with {} token(s)", validator.len());
        Router::new(config).with_validator(validator)
    } else {
        tracing::info!("Security mode: Open (no authentication)");
        Router::new(config)
    };

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
            let (cert_der, key_der) =
                if let (Some(cert_path), Some(key_path)) = (&cli.cert, &cli.key) {
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
    use rcgen::{generate_simple_self_signed, CertifiedKey};

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    tracing::debug!(
        "Generated self-signed certificate ({} bytes)",
        cert_der.len()
    );
    Ok((cert_der, key_der))
}
