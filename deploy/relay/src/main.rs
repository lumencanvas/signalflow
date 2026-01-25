//! CLASP Relay Server (Multi-Protocol)
//!
//! A CLASP relay server supporting multiple protocols:
//! - WebSocket (default, port 7330)
//! - QUIC (optional, port 7331)
//! - MQTT (optional, port 1883)
//! - OSC (optional, port 8000)
//!
//! All protocols share the same router state, allowing cross-protocol communication.
//!
//! # Usage
//!
//! ```bash
//! # Default (WebSocket only on port 7330)
//! clasp-relay
//!
//! # WebSocket + MQTT
//! clasp-relay --mqtt-port 1883
//!
//! # WebSocket + QUIC (requires cert/key)
//! clasp-relay --quic-port 7331 --cert cert.pem --key key.pem
//!
//! # All protocols
//! clasp-relay --mqtt-port 1883 --osc-port 8000 --quic-port 7331 --cert cert.pem --key key.pem
//! ```

use anyhow::Result;
use clap::Parser;
use clasp_core::SecurityMode;
use clasp_router::{MultiProtocolConfig, Router, RouterConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "clasp-relay")]
#[command(about = "CLASP Multi-Protocol Relay Server")]
#[command(version)]
struct Cli {
    /// WebSocket listen port (default: 7330)
    #[arg(short = 'p', long = "ws-port", default_value = "7330")]
    ws_port: u16,

    /// Listen host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Server name (shown in WELCOME)
    #[arg(short, long, default_value = "CLASP Relay")]
    name: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// QUIC listen port (enables QUIC transport, requires --cert and --key)
    #[arg(long)]
    quic_port: Option<u16>,

    /// MQTT listen port (enables MQTT server adapter)
    #[arg(long)]
    mqtt_port: Option<u16>,

    /// MQTT namespace prefix (default: /mqtt)
    #[arg(long, default_value = "/mqtt")]
    mqtt_namespace: String,

    /// OSC listen port (enables OSC server adapter)
    #[arg(long)]
    osc_port: Option<u16>,

    /// OSC namespace prefix (default: /osc)
    #[arg(long, default_value = "/osc")]
    osc_namespace: String,

    /// TLS certificate file (PEM format, for QUIC and MQTTS)
    #[arg(long)]
    cert: Option<PathBuf>,

    /// TLS private key file (PEM format, for QUIC and MQTTS)
    #[arg(long)]
    key: Option<PathBuf>,

    /// Maximum clients (0 = unlimited)
    #[arg(long, default_value = "1000")]
    max_sessions: usize,

    /// Session timeout in seconds
    #[arg(long, default_value = "300")]
    session_timeout: u64,

    /// Disable WebSocket (use other protocols only)
    #[arg(long)]
    no_websocket: bool,
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

    tracing::info!("╔══════════════════════════════════════════════════════════════╗");
    tracing::info!("║           CLASP Multi-Protocol Relay Server                  ║");
    tracing::info!("╚══════════════════════════════════════════════════════════════╝");

    // Create router configuration
    let config = RouterConfig {
        name: cli.name.clone(),
        security_mode: SecurityMode::Open,
        max_sessions: cli.max_sessions,
        session_timeout: cli.session_timeout,
        features: vec![
            "param".to_string(),
            "event".to_string(),
            "stream".to_string(),
            "timeline".to_string(),
            "gesture".to_string(),
        ],
        max_subscriptions_per_session: 100,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        max_messages_per_second: 0, // No rate limiting for public relay
        rate_limiting_enabled: false,
    };

    let router = Router::new(config);

    // Build multi-protocol configuration
    let mut protocols = Vec::new();

    // WebSocket (default)
    #[cfg(feature = "websocket")]
    let websocket_addr = if !cli.no_websocket {
        let addr = format!("{}:{}", cli.host, cli.ws_port);
        tracing::info!("WebSocket: ws://{}", addr);
        protocols.push("WebSocket");
        Some(addr)
    } else {
        None
    };

    #[cfg(not(feature = "websocket"))]
    let websocket_addr: Option<String> = None;

    // QUIC
    #[cfg(feature = "quic")]
    let quic_config = if let Some(quic_port) = cli.quic_port {
        let cert_path = cli
            .cert
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--cert required for QUIC"))?;
        let key_path = cli
            .key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--key required for QUIC"))?;

        // Load certificate and key
        let cert_pem = std::fs::read(cert_path)?;
        let key_pem = std::fs::read(key_path)?;

        // Parse PEM to DER
        let cert_der = rustls_pemfile::certs(&mut cert_pem.as_slice())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No certificate found in PEM file"))?
            .to_vec();

        let key_der = rustls_pemfile::private_key(&mut key_pem.as_slice())?
            .ok_or_else(|| anyhow::anyhow!("No private key found in PEM file"))?
            .secret_der()
            .to_vec();

        let addr: SocketAddr = format!("{}:{}", cli.host, quic_port).parse()?;
        tracing::info!("QUIC: {}", addr);
        protocols.push("QUIC");

        Some(clasp_router::QuicServerConfig {
            addr,
            cert: cert_der,
            key: key_der,
        })
    } else {
        None
    };

    #[cfg(not(feature = "quic"))]
    let quic_config: Option<()> = None;

    // MQTT
    #[cfg(feature = "mqtt-server")]
    let mqtt_config = if let Some(mqtt_port) = cli.mqtt_port {
        let addr = format!("{}:{}", cli.host, mqtt_port);
        tracing::info!("MQTT: mqtt://{} (namespace: {})", addr, cli.mqtt_namespace);
        protocols.push("MQTT");

        Some(clasp_router::MqttServerConfig {
            bind_addr: addr,
            namespace: cli.mqtt_namespace.clone(),
            require_auth: false,
            tls: None,
            max_clients: cli.max_sessions,
            session_timeout_secs: cli.session_timeout,
        })
    } else {
        None
    };

    #[cfg(not(feature = "mqtt-server"))]
    let mqtt_config: Option<()> = None;

    // OSC
    #[cfg(feature = "osc-server")]
    let osc_config = if let Some(osc_port) = cli.osc_port {
        let addr = format!("{}:{}", cli.host, osc_port);
        tracing::info!("OSC: udp://{} (namespace: {})", addr, cli.osc_namespace);
        protocols.push("OSC");

        Some(clasp_router::OscServerConfig {
            bind_addr: addr,
            namespace: cli.osc_namespace.clone(),
            session_timeout_secs: 30,
            auto_subscribe: false,
        })
    } else {
        None
    };

    #[cfg(not(feature = "osc-server"))]
    let osc_config: Option<()> = None;

    if protocols.is_empty() {
        anyhow::bail!("No protocols enabled. Enable at least one of: WebSocket, QUIC, MQTT, OSC");
    }

    tracing::info!("Server name: {}", cli.name);
    tracing::info!("Protocols: {}", protocols.join(", "));
    tracing::info!(
        "Max sessions: {}, Timeout: {}s",
        cli.max_sessions,
        cli.session_timeout
    );
    tracing::info!("────────────────────────────────────────────────────────────────");

    // Create multi-protocol config
    let multi_config = MultiProtocolConfig {
        #[cfg(feature = "websocket")]
        websocket_addr,
        #[cfg(feature = "quic")]
        quic: quic_config,
        #[cfg(feature = "mqtt-server")]
        mqtt: mqtt_config,
        #[cfg(feature = "osc-server")]
        osc: osc_config,
    };

    tracing::info!("Router initialized, accepting connections...");

    // Serve all protocols
    router.serve_all(multi_config).await?;

    Ok(())
}
