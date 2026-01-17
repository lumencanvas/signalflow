//! QUIC Transport Tests (clasp-transport)
//!
//! Tests for the QUIC transport implementation including:
//! - Configuration
//! - Client/Server creation
//! - Connection establishment
//! - Stream operations
//! - Datagram support
//!
//! Note: These tests require the 'quic' feature to be enabled

use std::time::Duration;

// ============================================================================
// Test Framework
// ============================================================================

struct TestResult {
    name: &'static str,
    passed: bool,
    message: String,
    duration_ms: u128,
}

impl TestResult {
    fn pass(name: &'static str, duration_ms: u128) -> Self {
        Self {
            name,
            passed: true,
            message: "OK".to_string(),
            duration_ms,
        }
    }

    fn fail(name: &'static str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name,
            passed: false,
            message: message.into(),
            duration_ms,
        }
    }

    fn skip(name: &'static str, reason: &str, duration_ms: u128) -> Self {
        Self {
            name,
            passed: true, // Skipped tests count as pass
            message: format!("SKIP: {}", reason),
            duration_ms,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

#[cfg(feature = "quic")]
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), String> {
    use rcgen::{CertifiedKey, generate_simple_self_signed};

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| format!("Cert generation failed: {}", e))?;

    Ok((cert.der().to_vec(), key_pair.serialize_der()))
}

#[cfg(feature = "quic")]
async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

// ============================================================================
// Configuration Tests
// ============================================================================

fn test_quic_config_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_config_default";

    #[cfg(feature = "quic")]
    {
        use clasp_transport::quic::QuicConfig;

        let config = QuicConfig::default();

        if config.enable_0rtt
            && config.keep_alive_ms == 5000
            && config.idle_timeout_ms == 30000
            && config.initial_window == 10
        {
            return TestResult::pass(name, start.elapsed().as_millis());
        }
        return TestResult::fail(name, "Default config incorrect", start.elapsed().as_millis());
    }

    #[cfg(not(feature = "quic"))]
    TestResult::skip(name, "QUIC feature not enabled", start.elapsed().as_millis())
}

fn test_quic_config_custom() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_config_custom";

    #[cfg(feature = "quic")]
    {
        use clasp_transport::quic::QuicConfig;

        let config = QuicConfig {
            enable_0rtt: false,
            keep_alive_ms: 10000,
            idle_timeout_ms: 60000,
            initial_window: 20,
        };

        if !config.enable_0rtt
            && config.keep_alive_ms == 10000
            && config.idle_timeout_ms == 60000
            && config.initial_window == 20
        {
            return TestResult::pass(name, start.elapsed().as_millis());
        }
        return TestResult::fail(name, "Custom config not set correctly", start.elapsed().as_millis());
    }

    #[cfg(not(feature = "quic"))]
    TestResult::skip(name, "QUIC feature not enabled", start.elapsed().as_millis())
}

// ============================================================================
// Client Creation Tests
// ============================================================================

#[cfg(feature = "quic")]
async fn test_quic_client_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_client_creation";

    use clasp_transport::quic::QuicTransport;

    match QuicTransport::new_client() {
        Ok(_client) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("Client creation failed: {}", e), start.elapsed().as_millis()),
    }
}

#[cfg(not(feature = "quic"))]
async fn test_quic_client_creation() -> TestResult {
    let start = std::time::Instant::now();
    TestResult::skip("quic_client_creation", "QUIC feature not enabled", start.elapsed().as_millis())
}

#[cfg(feature = "quic")]
async fn test_quic_client_with_config() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_client_with_config";

    use clasp_transport::quic::{QuicConfig, QuicTransport};

    let config = QuicConfig {
        enable_0rtt: true,
        keep_alive_ms: 3000,
        idle_timeout_ms: 15000,
        initial_window: 5,
    };

    match QuicTransport::new_client_with_config(config) {
        Ok(_client) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("Client creation failed: {}", e), start.elapsed().as_millis()),
    }
}

#[cfg(not(feature = "quic"))]
async fn test_quic_client_with_config() -> TestResult {
    let start = std::time::Instant::now();
    TestResult::skip("quic_client_with_config", "QUIC feature not enabled", start.elapsed().as_millis())
}

// ============================================================================
// Server Creation Tests
// ============================================================================

#[cfg(feature = "quic")]
async fn test_quic_server_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_server_creation";

    use clasp_transport::quic::QuicTransport;
    use std::net::SocketAddr;

    let port = find_available_port().await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let (cert, key) = match generate_self_signed_cert() {
        Ok((c, k)) => (c, k),
        Err(e) => return TestResult::fail(name, format!("Cert generation failed: {}", e), start.elapsed().as_millis()),
    };

    match QuicTransport::new_server(addr, cert, key) {
        Ok(server) => {
            // Verify we can get the local address
            match server.local_addr() {
                Ok(local) => {
                    if local.port() == port {
                        TestResult::pass(name, start.elapsed().as_millis())
                    } else {
                        TestResult::fail(name, format!("Wrong port: {} vs {}", local.port(), port), start.elapsed().as_millis())
                    }
                }
                Err(e) => TestResult::fail(name, format!("Failed to get local addr: {}", e), start.elapsed().as_millis()),
            }
        }
        Err(e) => TestResult::fail(name, format!("Server creation failed: {}", e), start.elapsed().as_millis()),
    }
}

#[cfg(not(feature = "quic"))]
async fn test_quic_server_creation() -> TestResult {
    let start = std::time::Instant::now();
    TestResult::skip("quic_server_creation", "QUIC feature not enabled", start.elapsed().as_millis())
}

// ============================================================================
// Connection Tests
// ============================================================================

#[cfg(feature = "quic")]
async fn test_quic_client_server_connect() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_client_server_connect";

    use clasp_transport::quic::QuicTransport;
    use std::net::SocketAddr;

    let port = find_available_port().await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let (cert, key) = match generate_self_signed_cert() {
        Ok((c, k)) => (c, k),
        Err(e) => return TestResult::fail(name, format!("Cert generation failed: {}", e), start.elapsed().as_millis()),
    };

    // Create server
    let server = match QuicTransport::new_server(addr, cert, key) {
        Ok(s) => s,
        Err(e) => return TestResult::fail(name, format!("Server creation failed: {}", e), start.elapsed().as_millis()),
    };

    // Create client
    let client = match QuicTransport::new_client() {
        Ok(c) => c,
        Err(e) => return TestResult::fail(name, format!("Client creation failed: {}", e), start.elapsed().as_millis()),
    };

    // Spawn server accept task
    let server_handle = tokio::spawn(async move {
        match tokio::time::timeout(Duration::from_secs(5), server.accept()).await {
            Ok(Ok(conn)) => {
                let remote = conn.remote_address();
                tracing::info!("Server accepted connection from {}", remote);
                Ok(conn)
            }
            Ok(Err(e)) => Err(format!("Accept failed: {}", e)),
            Err(_) => Err("Accept timeout".to_string()),
        }
    });

    // Give server time to start listening
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect from client
    let client_result = client.connect(addr, "localhost").await;

    // Wait for server accept
    let server_result = server_handle.await;

    match (client_result, server_result) {
        (Ok(client_conn), Ok(Ok(server_conn))) => {
            // Verify both connections are established
            let client_remote = client_conn.remote_address();
            let server_remote = server_conn.remote_address();

            if client_remote.port() == port {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, format!("Client connected to wrong port: {}", client_remote.port()), start.elapsed().as_millis())
            }
        }
        (Err(e), _) => TestResult::fail(name, format!("Client connect failed: {}", e), start.elapsed().as_millis()),
        (_, Ok(Err(e))) => TestResult::fail(name, format!("Server accept failed: {}", e), start.elapsed().as_millis()),
        (_, Err(e)) => TestResult::fail(name, format!("Server task failed: {}", e), start.elapsed().as_millis()),
    }
}

#[cfg(not(feature = "quic"))]
async fn test_quic_client_server_connect() -> TestResult {
    let start = std::time::Instant::now();
    TestResult::skip("quic_client_server_connect", "QUIC feature not enabled", start.elapsed().as_millis())
}

// ============================================================================
// Stream Tests
// ============================================================================

#[cfg(feature = "quic")]
async fn test_quic_bidirectional_stream() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_bidirectional_stream";

    use bytes::Bytes;
    use clasp_transport::quic::QuicTransport;
    use clasp_transport::TransportSender;
    use std::net::SocketAddr;

    let port = find_available_port().await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let (cert, key) = match generate_self_signed_cert() {
        Ok((c, k)) => (c, k),
        Err(e) => return TestResult::fail(name, format!("Cert generation failed: {}", e), start.elapsed().as_millis()),
    };

    let server = match QuicTransport::new_server(addr, cert, key) {
        Ok(s) => s,
        Err(e) => return TestResult::fail(name, format!("Server creation failed: {}", e), start.elapsed().as_millis()),
    };

    let client = match QuicTransport::new_client() {
        Ok(c) => c,
        Err(e) => return TestResult::fail(name, format!("Client creation failed: {}", e), start.elapsed().as_millis()),
    };

    // Server task
    let server_handle = tokio::spawn(async move {
        let conn = server.accept().await?;
        let (sender, mut receiver) = conn.accept_bi().await?;

        // Read message from client
        use clasp_transport::TransportReceiver;
        match receiver.recv().await {
            Some(clasp_transport::TransportEvent::Data(data)) => {
                // Echo back
                sender.send(data).await?;
                Ok::<_, clasp_transport::TransportError>(())
            }
            other => Err(clasp_transport::TransportError::ConnectionFailed(format!("Unexpected: {:?}", other))),
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client connects and sends
    let result: Result<(), String> = async {
        let conn = client.connect(addr, "localhost").await.map_err(|e| e.to_string())?;

        let (sender, mut receiver) = conn.open_bi().await.map_err(|e| e.to_string())?;

        // Send test message
        let test_data = Bytes::from_static(b"Hello QUIC!");
        sender.send(test_data.clone()).await.map_err(|e| e.to_string())?;

        // Wait for echo
        use clasp_transport::TransportReceiver;
        match tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
            Ok(Some(clasp_transport::TransportEvent::Data(data))) => {
                if data == test_data {
                    Ok(())
                } else {
                    Err(format!("Data mismatch: {:?} vs {:?}", data, test_data))
                }
            }
            Ok(other) => Err(format!("Unexpected event: {:?}", other)),
            Err(_) => Err("Timeout waiting for echo".to_string()),
        }
    }
    .await;

    // Clean up server
    let _ = server_handle.await;

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

#[cfg(not(feature = "quic"))]
async fn test_quic_bidirectional_stream() -> TestResult {
    let start = std::time::Instant::now();
    TestResult::skip("quic_bidirectional_stream", "QUIC feature not enabled", start.elapsed().as_millis())
}

// ============================================================================
// ALPN Test
// ============================================================================

fn test_quic_alpn_protocol() -> TestResult {
    let start = std::time::Instant::now();
    let name = "quic_alpn_protocol";

    #[cfg(feature = "quic")]
    {
        use clasp_transport::quic::CLASP_ALPN;

        if CLASP_ALPN == b"clasp/2" {
            return TestResult::pass(name, start.elapsed().as_millis());
        }
        return TestResult::fail(name, format!("Wrong ALPN: {:?}", CLASP_ALPN), start.elapsed().as_millis());
    }

    #[cfg(not(feature = "quic"))]
    TestResult::skip(name, "QUIC feature not enabled", start.elapsed().as_millis())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP QUIC Transport Tests                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    #[cfg(not(feature = "quic"))]
    println!("Note: QUIC feature not enabled. Tests will be skipped.\n");

    let tests = vec![
        // Configuration tests
        test_quic_config_default(),
        test_quic_config_custom(),
        test_quic_alpn_protocol(),

        // Client creation tests
        test_quic_client_creation().await,
        test_quic_client_with_config().await,

        // Server creation tests
        test_quic_server_creation().await,

        // Connection tests
        test_quic_client_server_connect().await,

        // Stream tests
        test_quic_bidirectional_stream().await,
    ];

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    println!("┌──────────────────────────────────────┬────────┬──────────┐");
    println!("│ Test                                 │ Status │ Time     │");
    println!("├──────────────────────────────────────┼────────┼──────────┤");

    for test in &tests {
        let (status, color) = if test.message.starts_with("SKIP") {
            skipped += 1;
            ("○ SKIP", "\x1b[33m") // Yellow
        } else if test.passed {
            passed += 1;
            ("✓ PASS", "\x1b[32m") // Green
        } else {
            failed += 1;
            ("✗ FAIL", "\x1b[31m") // Red
        };

        println!(
            "│ {:<36} │ {}{:<6}\x1b[0m │ {:>6}ms │",
            test.name, color, status, test.duration_ms
        );

        if !test.passed && !test.message.starts_with("SKIP") {
            println!("│   └─ {:<56} │", &test.message[..test.message.len().min(56)]);
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed, {} skipped", passed, failed, skipped);

    if failed > 0 {
        std::process::exit(1);
    }
}
