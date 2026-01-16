//! CLASP Server implementation for CLI

use anyhow::Result;
use colored::Colorize;
use tokio::sync::mpsc;

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
    use clasp_transport::quic::{QuicConfig, QuicTransport};
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("{}:{}", bind, port).parse()?;

    // Generate self-signed certificate for development
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_der = cert.serialize_der()?;
    let key_der = cert.serialize_private_key_der();

    let config = QuicConfig::default();
    let transport = QuicTransport::new_server_with_config(addr, cert_der, key_der, config)?;

    println!(
        "{} QUIC server listening on {}:{}",
        "OK".green().bold(),
        bind,
        port
    );
    println!("  Protocol:  CLASP over QUIC (TLS 1.3)");
    println!("  TLS:       Self-signed certificate (dev mode)");
    println!("  ALPN:      clasp/2");
    println!("  Press Ctrl+C to stop");

    // Accept connections
    loop {
        tokio::select! {
            result = transport.accept() => {
                match result {
                    Ok(conn) => {
                        let remote = conn.remote_address();
                        println!("{} Client connected: {}", "QUIC".cyan(), remote);
                        tokio::spawn(async move {
                            if let Ok((_, mut rx)) = conn.accept_bi().await {
                                use clasp_transport::TransportReceiver;
                                while let Some(event) = rx.recv().await {
                                    println!("{} {:?}", "QUIC".cyan(), event);
                                }
                            }
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
