//! CLASP CLI - Command-line interface for CLASP protocol servers and bridges
//!
//! Start protocol servers, bridges, and manage CLASP signals from the command line.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod server;

/// CLASP - Creative Low-Latency Application Streaming Protocol
#[derive(Parser)]
#[command(name = "clasp")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, global = true, default_value = "info")]
    log_level: String,

    /// Output logs as JSON
    #[arg(long, global = true)]
    json_logs: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a CLASP server
    Server {
        /// Protocol to serve (quic, tcp, websocket)
        #[arg(short, long, default_value = "quic")]
        protocol: String,

        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,

        /// Port number
        #[arg(short = 'P', long, default_value = "7331")]
        port: u16,
    },

    /// Start a protocol bridge
    Bridge {
        /// Bridge type (osc, midi, artnet, mqtt, websocket, http)
        #[arg(short, long)]
        bridge_type: String,

        /// Configuration options (key=value pairs)
        #[arg(short, long)]
        opt: Vec<String>,
    },

    /// Start an OSC server
    Osc {
        /// UDP port to listen on
        #[arg(short, long, default_value = "9000")]
        port: u16,

        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
    },

    /// Start an MQTT broker connection
    Mqtt {
        /// MQTT broker host
        #[arg(short = 'H', long, default_value = "localhost")]
        host: String,

        /// MQTT broker port
        #[arg(short, long, default_value = "1883")]
        port: u16,

        /// Client ID
        #[arg(short, long)]
        client_id: Option<String>,

        /// Topics to subscribe (supports wildcards)
        #[arg(short, long, default_value = "#")]
        topic: Vec<String>,
    },

    /// Start a WebSocket server or client
    Websocket {
        /// Mode: server or client
        #[arg(short, long, default_value = "server")]
        mode: String,

        /// URL (ws://... for client) or bind address for server
        #[arg(short, long, default_value = "0.0.0.0:8080")]
        url: String,
    },

    /// Start an HTTP REST API server
    Http {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0:3000")]
        bind: String,

        /// Base path for API endpoints
        #[arg(short, long, default_value = "/api")]
        base_path: String,

        /// Enable CORS
        #[arg(long, default_value = "true")]
        cors: bool,
    },

    /// Publish a value to a CLASP address
    Pub {
        /// CLASP server URL
        #[arg(short, long, default_value = "quic://localhost:7331")]
        server: String,

        /// Signal address
        address: String,

        /// Value to publish (JSON format)
        value: String,
    },

    /// Subscribe to signals
    Sub {
        /// CLASP server URL
        #[arg(short, long, default_value = "quic://localhost:7331")]
        server: String,

        /// Address pattern to subscribe to
        #[arg(default_value = "/**")]
        pattern: String,
    },

    /// Show version and system info
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    setup_logging(&cli.log_level, cli.json_logs)?;

    // Handle Ctrl+C
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
        info!("Received shutdown signal");
        let _ = shutdown_tx_clone.send(()).await;
    });

    match cli.command {
        Commands::Server {
            protocol,
            bind,
            port,
        } => {
            server::run_server(&protocol, &bind, port, &mut shutdown_rx).await?;
        }

        Commands::Bridge { bridge_type, opt } => {
            run_bridge(&bridge_type, opt, &mut shutdown_rx).await?;
        }

        Commands::Osc { port, bind } => {
            println!(
                "{} Starting OSC server on {}:{}",
                "CLASP".cyan().bold(),
                bind,
                port
            );
            run_osc_server(&bind, port, &mut shutdown_rx).await?;
        }

        Commands::Mqtt {
            host,
            port,
            client_id,
            topic,
        } => {
            println!(
                "{} Connecting to MQTT broker at {}:{}",
                "CLASP".cyan().bold(),
                host,
                port
            );
            run_mqtt_bridge(&host, port, client_id, topic, &mut shutdown_rx).await?;
        }

        Commands::Websocket { mode, url } => {
            println!(
                "{} Starting WebSocket {} on {}",
                "CLASP".cyan().bold(),
                mode,
                url
            );
            run_websocket(&mode, &url, &mut shutdown_rx).await?;
        }

        Commands::Http {
            bind,
            base_path,
            cors,
        } => {
            println!(
                "{} Starting HTTP server on {} (base: {})",
                "CLASP".cyan().bold(),
                bind,
                base_path
            );
            run_http_server(&bind, &base_path, cors, &mut shutdown_rx).await?;
        }

        Commands::Pub {
            server,
            address,
            value,
        } => {
            println!(
                "{} Publishing to {} -> {}",
                "CLASP".cyan().bold(),
                address.yellow(),
                value
            );
            publish_value(&server, &address, &value).await?;
        }

        Commands::Sub { server, pattern } => {
            println!(
                "{} Subscribing to {} on {}",
                "CLASP".cyan().bold(),
                pattern.yellow(),
                server
            );
            subscribe_pattern(&server, &pattern, &mut shutdown_rx).await?;
        }

        Commands::Info => {
            print_info();
        }
    }

    Ok(())
}

fn setup_logging(level: &str, json: bool) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .context("Failed to parse log level")?;

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(false).compact())
            .init();
    }

    Ok(())
}

async fn run_bridge(
    bridge_type: &str,
    opts: Vec<String>,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    println!(
        "{} Starting {} bridge",
        "CLASP".cyan().bold(),
        bridge_type.green()
    );

    // Parse options into a map
    let _options: std::collections::HashMap<String, String> = opts
        .iter()
        .filter_map(|opt| {
            let parts: Vec<&str> = opt.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    match bridge_type {
        "osc" => {
            println!("  Use 'clasp osc' for OSC-specific options");
        }
        "mqtt" => {
            println!("  Use 'clasp mqtt' for MQTT-specific options");
        }
        "websocket" | "ws" => {
            println!("  Use 'clasp websocket' for WebSocket-specific options");
        }
        "http" => {
            println!("  Use 'clasp http' for HTTP-specific options");
        }
        _ => {
            println!("{}", format!("Unknown bridge type: {}", bridge_type).red());
            return Ok(());
        }
    }

    // Wait for shutdown
    shutdown_rx.recv().await;
    println!("{}", "Bridge stopped".yellow());

    Ok(())
}

async fn run_osc_server(bind: &str, port: u16, shutdown_rx: &mut mpsc::Receiver<()>) -> Result<()> {
    use clasp_bridge::{Bridge, OscBridge, OscBridgeConfig};

    let config = OscBridgeConfig {
        bind_addr: format!("{}:{}", bind, port),
        ..Default::default()
    };

    let mut bridge = OscBridge::new(config);
    let mut event_rx = bridge.start().await?;

    println!("{} OSC server listening", "OK".green().bold());

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    println!("{} {:?}", "OSC".cyan(), event);
                }
            }
            _ = shutdown_rx.recv() => {
                bridge.stop().await?;
                break;
            }
        }
    }

    Ok(())
}

async fn run_mqtt_bridge(
    host: &str,
    port: u16,
    client_id: Option<String>,
    topics: Vec<String>,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    use clasp_bridge::{Bridge, MqttBridge, MqttBridgeConfig};

    let config = MqttBridgeConfig {
        broker_host: host.to_string(),
        broker_port: port,
        client_id: client_id.unwrap_or_else(|| format!("clasp-cli-{}", std::process::id())),
        subscribe_topics: topics,
        ..Default::default()
    };

    let mut bridge = MqttBridge::new(config);
    let mut event_rx = bridge.start().await?;

    println!("{} MQTT bridge connected", "OK".green().bold());

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    println!("{} {:?}", "MQTT".cyan(), event);
                }
            }
            _ = shutdown_rx.recv() => {
                bridge.stop().await?;
                break;
            }
        }
    }

    Ok(())
}

async fn run_websocket(mode: &str, url: &str, shutdown_rx: &mut mpsc::Receiver<()>) -> Result<()> {
    use clasp_bridge::{Bridge, WebSocketBridge, WebSocketBridgeConfig, WsMode};

    let ws_mode = match mode {
        "server" => WsMode::Server,
        "client" => WsMode::Client,
        _ => {
            println!("{}", "Mode must be 'server' or 'client'".red());
            return Ok(());
        }
    };

    let config = WebSocketBridgeConfig {
        mode: ws_mode,
        url: url.to_string(),
        ..Default::default()
    };

    let mut bridge = WebSocketBridge::new(config);
    let mut event_rx = bridge.start().await?;

    println!("{} WebSocket {} started", "OK".green().bold(), mode);

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    println!("{} {:?}", "WS".cyan(), event);
                }
            }
            _ = shutdown_rx.recv() => {
                bridge.stop().await?;
                break;
            }
        }
    }

    Ok(())
}

async fn run_http_server(
    bind: &str,
    base_path: &str,
    cors: bool,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    use clasp_bridge::{Bridge, HttpBridge, HttpBridgeConfig, HttpMode};

    let config = HttpBridgeConfig {
        mode: HttpMode::Server,
        url: bind.to_string(),
        base_path: base_path.to_string(),
        cors_enabled: cors,
        ..Default::default()
    };

    let mut bridge = HttpBridge::new(config);
    let mut event_rx = bridge.start().await?;

    println!("{} HTTP server started", "OK".green().bold());
    println!("  Endpoints:");
    println!("    GET  {}/signals       - List all signals", base_path);
    println!("    GET  {}/*path         - Get signal value", base_path);
    println!("    PUT  {}/*path         - Set signal value", base_path);
    println!("    POST {}/*path         - Publish event", base_path);
    println!("    GET  {}/health        - Health check", base_path);

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    println!("{} {:?}", "HTTP".cyan(), event);
                }
            }
            _ = shutdown_rx.recv() => {
                bridge.stop().await?;
                break;
            }
        }
    }

    Ok(())
}

async fn publish_value(_server: &str, address: &str, value: &str) -> Result<()> {
    // Parse value as JSON
    let parsed: serde_json::Value = serde_json::from_str(value)
        .or_else(|_| Ok::<_, serde_json::Error>(serde_json::Value::String(value.to_string())))?;

    println!(
        "{} Published {} = {}",
        "OK".green().bold(),
        address.yellow(),
        serde_json::to_string_pretty(&parsed)?
    );

    // TODO: Connect to CLASP server and publish
    warn!("Server connection not yet implemented");

    Ok(())
}

async fn subscribe_pattern(
    _server: &str,
    pattern: &str,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    println!(
        "{} Subscribed to pattern: {}",
        "OK".green().bold(),
        pattern.yellow()
    );

    // TODO: Connect to CLASP server and subscribe
    warn!("Server connection not yet implemented - press Ctrl+C to exit");

    shutdown_rx.recv().await;

    Ok(())
}

fn print_info() {
    println!(
        "{}",
        "CLASP - Creative Low-Latency Application Streaming Protocol"
            .cyan()
            .bold()
    );
    println!();
    println!("Version:    {}", env!("CARGO_PKG_VERSION"));
    println!("Platform:   {}", std::env::consts::OS);
    println!("Arch:       {}", std::env::consts::ARCH);
    println!();
    println!("{}", "Supported Protocols:".green());
    println!("  - CLASP/QUIC (native, low-latency)");
    println!("  - OSC (Open Sound Control)");
    println!("  - MIDI (Musical Instrument Digital Interface)");
    println!("  - Art-Net (Ethernet DMX)");
    println!("  - MQTT (IoT messaging)");
    println!("  - WebSocket (bidirectional web)");
    println!("  - HTTP/REST (request-response API)");
    println!();
    println!("{}", "Examples:".green());
    println!("  clasp osc --port 9000            # Start OSC server");
    println!("  clasp mqtt --host broker.local   # Connect to MQTT broker");
    println!("  clasp http --bind 0.0.0.0:3000   # Start HTTP REST API");
    println!("  clasp websocket --mode server    # Start WebSocket server");
}
