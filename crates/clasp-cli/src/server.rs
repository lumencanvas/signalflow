//! CLASP Server implementation for CLI

use anyhow::Result;
use colored::Colorize;
use tokio::sync::mpsc;
use tracing::info;

/// Run a CLASP protocol server
pub async fn run_server(
    protocol: &str,
    bind: &str,
    port: u16,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    println!(
        "{} Starting {} server on {}:{}",
        "CLASP".cyan().bold(),
        protocol.green(),
        bind,
        port
    );

    match protocol {
        "quic" => run_quic_server(bind, port, shutdown_rx).await,
        "tcp" => run_tcp_server(bind, port, shutdown_rx).await,
        "websocket" | "ws" => run_ws_server(bind, port, shutdown_rx).await,
        _ => {
            println!(
                "{}",
                format!(
                    "Unknown protocol: {}. Use quic, tcp, or websocket.",
                    protocol
                )
                .red()
            );
            Ok(())
        }
    }
}

async fn run_quic_server(
    bind: &str,
    port: u16,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    #[cfg(feature = "quic")]
    {
        use clasp_transport::{QuicConfig, QuicTransport};

        let config = QuicConfig::default();
        let addr = format!("{}:{}", bind, port);

        let transport = QuicTransport::bind(&addr, config).await?;

        println!(
            "{} QUIC server listening on {}:{}",
            "OK".green().bold(),
            bind,
            port
        );
        println!("  Protocol:  CLASP over QUIC");
        println!("  TLS:       Self-signed certificate");
        println!("  Press Ctrl+C to stop");

        // Run until shutdown
        shutdown_rx.recv().await;
        info!("QUIC server shutting down");

        drop(transport);
        println!("{}", "Server stopped".yellow());

        Ok(())
    }

    #[cfg(not(feature = "quic"))]
    {
        println!(
            "{}",
            "QUIC support not compiled. Rebuild with --features quic".red()
        );
        println!(
            "{} Falling back to WebSocket server on {}:{}",
            "INFO".cyan(),
            bind,
            port
        );
        run_ws_server(bind, port, shutdown_rx).await
    }
}

async fn run_tcp_server(bind: &str, port: u16, shutdown_rx: &mut mpsc::Receiver<()>) -> Result<()> {
    use tokio::net::TcpListener;

    let addr = format!("{}:{}", bind, port);
    let listener = TcpListener::bind(&addr).await?;

    println!("{} TCP server listening on {}", "OK".green().bold(), addr);
    println!("  Protocol:  CLASP over TCP");
    println!("  Press Ctrl+C to stop");

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                        println!("{} Client connected: {}", "TCP".cyan(), peer);
                        tokio::spawn(async move {
                            // Handle connection
                            let _ = handle_tcp_connection(stream).await;
                        });
                    }
                    Err(e) => {
                        println!("{} Accept error: {}", "ERROR".red(), e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }

    println!("{}", "Server stopped".yellow());
    Ok(())
}

async fn handle_tcp_connection(mut stream: tokio::net::TcpStream) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = [0u8; 4096];

    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        // Try to decode CLASP message
        if let Ok((msg, _)) = clasp_core::codec::decode(&buf[..n]) {
            println!("{} Received: {:?}", "TCP".cyan(), msg);

            // Echo back for now
            let response = clasp_core::codec::encode(&msg)?;
            stream.write_all(&response).await?;
        }
    }

    Ok(())
}

async fn run_ws_server(bind: &str, port: u16, shutdown_rx: &mut mpsc::Receiver<()>) -> Result<()> {
    use clasp_bridge::{Bridge, WebSocketBridge, WebSocketBridgeConfig, WsMode};

    let config = WebSocketBridgeConfig {
        mode: WsMode::Server,
        url: format!("{}:{}", bind, port),
        ..Default::default()
    };

    let mut bridge = WebSocketBridge::new(config);
    let mut event_rx = bridge.start().await?;

    println!(
        "{} WebSocket server listening on {}:{}",
        "OK".green().bold(),
        bind,
        port
    );
    println!("  Protocol:  CLASP over WebSocket");
    println!("  Format:    JSON");
    println!("  Press Ctrl+C to stop");

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

    println!("{}", "Server stopped".yellow());
    Ok(())
}
