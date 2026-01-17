//! UDP Transport Tests (clasp-transport)
//!
//! Tests for the UDP transport implementation including:
//! - Binding and local address
//! - Send/receive operations
//! - Broadcast functionality
//! - Multiple concurrent sockets

use bytes::Bytes;
use clasp_transport::udp::{UdpBroadcast, UdpConfig, UdpTransport};
use clasp_transport::{TransportEvent, TransportReceiver, TransportSender};
use std::net::SocketAddr;
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
}

// ============================================================================
// Basic Binding Tests
// ============================================================================

async fn test_udp_bind_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_bind_default";

    match UdpTransport::bind("127.0.0.1:0").await {
        Ok(transport) => {
            match transport.local_addr() {
                Ok(addr) => {
                    if addr.port() > 0 {
                        TestResult::pass(name, start.elapsed().as_millis())
                    } else {
                        TestResult::fail(name, "Port should be > 0", start.elapsed().as_millis())
                    }
                }
                Err(e) => TestResult::fail(name, format!("Failed to get local addr: {}", e), start.elapsed().as_millis()),
            }
        }
        Err(e) => TestResult::fail(name, format!("Bind failed: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_udp_bind_with_config() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_bind_with_config";

    let config = UdpConfig {
        recv_buffer_size: 32768,
        max_packet_size: 1500,
    };

    match UdpTransport::bind_with_config("127.0.0.1:0", config).await {
        Ok(transport) => {
            if transport.local_addr().is_ok() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Failed to get local addr", start.elapsed().as_millis())
            }
        }
        Err(e) => TestResult::fail(name, format!("Bind failed: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_udp_bind_specific_port() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_bind_specific_port";

    // Find an available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    match UdpTransport::bind(&format!("127.0.0.1:{}", port)).await {
        Ok(transport) => {
            match transport.local_addr() {
                Ok(addr) => {
                    if addr.port() == port {
                        TestResult::pass(name, start.elapsed().as_millis())
                    } else {
                        TestResult::fail(name, format!("Wrong port: {} != {}", addr.port(), port), start.elapsed().as_millis())
                    }
                }
                Err(e) => TestResult::fail(name, format!("Failed to get local addr: {}", e), start.elapsed().as_millis()),
            }
        }
        Err(e) => TestResult::fail(name, format!("Bind failed: {}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Send/Receive Tests
// ============================================================================

async fn test_udp_send_receive() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_send_receive";

    let server = match UdpTransport::bind("127.0.0.1:0").await {
        Ok(t) => t,
        Err(e) => return TestResult::fail(name, format!("Server bind failed: {}", e), start.elapsed().as_millis()),
    };

    let client = match UdpTransport::bind("127.0.0.1:0").await {
        Ok(t) => t,
        Err(e) => return TestResult::fail(name, format!("Client bind failed: {}", e), start.elapsed().as_millis()),
    };

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Send from client
    if let Err(e) = client.send_to(b"hello udp", server_addr).await {
        return TestResult::fail(name, format!("Send failed: {}", e), start.elapsed().as_millis());
    }

    // Receive with timeout
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;

    match result {
        Ok(Some((TransportEvent::Data(data), from))) => {
            if data.as_ref() == b"hello udp" && from.port() == client.local_addr().unwrap().port() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Data or source mismatch", start.elapsed().as_millis())
            }
        }
        Ok(Some((TransportEvent::Error(e), _))) => {
            TestResult::fail(name, format!("Receive error: {}", e), start.elapsed().as_millis())
        }
        Ok(None) => TestResult::fail(name, "Receiver closed", start.elapsed().as_millis()),
        Err(_) => TestResult::fail(name, "Timeout waiting for data", start.elapsed().as_millis()),
        _ => TestResult::fail(name, "Unexpected event", start.elapsed().as_millis()),
    }
}

async fn test_udp_sender_to() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_sender_to";

    let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Create a sender targeting the server
    let sender = client.sender_to(server_addr);

    // Verify sender is connected
    if !sender.is_connected() {
        return TestResult::fail(name, "Sender not connected", start.elapsed().as_millis());
    }

    // Send using TransportSender trait
    if let Err(e) = sender.send(Bytes::from_static(b"via sender")).await {
        return TestResult::fail(name, format!("Send failed: {}", e), start.elapsed().as_millis());
    }

    // Receive
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;

    match result {
        Ok(Some((TransportEvent::Data(data), _))) => {
            if data.as_ref() == b"via sender" {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Data mismatch", start.elapsed().as_millis())
            }
        }
        _ => TestResult::fail(name, "Failed to receive", start.elapsed().as_millis()),
    }
}

async fn test_udp_multiple_messages() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_multiple_messages";

    let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Send multiple messages
    for i in 0..10 {
        let msg = format!("message {}", i);
        client.send_to(msg.as_bytes(), server_addr).await.unwrap();
    }

    // Receive all messages
    let mut received = 0;
    for _ in 0..10 {
        let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;
        if let Ok(Some((TransportEvent::Data(_), _))) = result {
            received += 1;
        }
    }

    if received >= 8 { // Allow some packet loss
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, format!("Only received {}/10 messages", received), start.elapsed().as_millis())
    }
}

async fn test_udp_large_packet() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_large_packet";

    let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Send a reasonably large packet (8KB - safe for all platforms)
    let large_data = vec![0xAB; 8192];
    client.send_to(&large_data, server_addr).await.unwrap();

    // Receive
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;

    match result {
        Ok(Some((TransportEvent::Data(data), _))) => {
            if data.len() == 8192 && data[0] == 0xAB {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, format!("Data size/content mismatch: {}", data.len()), start.elapsed().as_millis())
            }
        }
        _ => TestResult::fail(name, "Failed to receive large packet", start.elapsed().as_millis()),
    }
}

// ============================================================================
// Broadcast Tests
// ============================================================================

async fn test_udp_set_broadcast() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_set_broadcast";

    let transport = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    match transport.set_broadcast(true) {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("Failed to enable broadcast: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_udp_broadcast_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_broadcast_creation";

    match UdpBroadcast::new(7331).await {
        Ok(_broadcast) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("Broadcast creation failed: {}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Concurrent Tests
// ============================================================================

async fn test_udp_concurrent_sockets() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_concurrent_sockets";

    // Create multiple sockets
    let mut sockets = vec![];
    for _ in 0..5 {
        match UdpTransport::bind("127.0.0.1:0").await {
            Ok(s) => sockets.push(s),
            Err(e) => return TestResult::fail(name, format!("Bind failed: {}", e), start.elapsed().as_millis()),
        }
    }

    // Verify all have unique ports
    let ports: Vec<u16> = sockets.iter().map(|s| s.local_addr().unwrap().port()).collect();
    let unique_ports: std::collections::HashSet<u16> = ports.iter().cloned().collect();

    if unique_ports.len() == 5 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Not all ports are unique", start.elapsed().as_millis())
    }
}

async fn test_udp_bidirectional() -> TestResult {
    let start = std::time::Instant::now();
    let name = "udp_bidirectional";

    let socket_a = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let socket_b = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let addr_a = socket_a.local_addr().unwrap();
    let addr_b = socket_b.local_addr().unwrap();

    let mut recv_a = socket_a.start_receiver();
    let mut recv_b = socket_b.start_receiver();

    // A sends to B
    socket_a.send_to(b"hello from A", addr_b).await.unwrap();

    // B receives and sends back
    if let Ok(Some((TransportEvent::Data(data), from))) =
        tokio::time::timeout(Duration::from_secs(2), recv_b.recv_from()).await {
        if data.as_ref() == b"hello from A" {
            socket_b.send_to(b"hello from B", from).await.unwrap();
        }
    } else {
        return TestResult::fail(name, "B didn't receive from A", start.elapsed().as_millis());
    }

    // A receives response
    if let Ok(Some((TransportEvent::Data(data), _))) =
        tokio::time::timeout(Duration::from_secs(2), recv_a.recv_from()).await {
        if data.as_ref() == b"hello from B" {
            return TestResult::pass(name, start.elapsed().as_millis());
        }
    }

    TestResult::fail(name, "A didn't receive from B", start.elapsed().as_millis())
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
    println!("║              CLASP UDP Transport Tests                           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        // Binding tests
        test_udp_bind_default().await,
        test_udp_bind_with_config().await,
        test_udp_bind_specific_port().await,

        // Send/Receive tests
        test_udp_send_receive().await,
        test_udp_sender_to().await,
        test_udp_multiple_messages().await,
        test_udp_large_packet().await,

        // Broadcast tests
        test_udp_set_broadcast().await,
        test_udp_broadcast_creation().await,

        // Concurrent tests
        test_udp_concurrent_sockets().await,
        test_udp_bidirectional().await,
    ];

    let mut passed = 0;
    let mut failed = 0;

    println!("┌──────────────────────────────────────┬────────┬──────────┐");
    println!("│ Test                                 │ Status │ Time     │");
    println!("├──────────────────────────────────────┼────────┼──────────┤");

    for test in &tests {
        let status = if test.passed { "✓ PASS" } else { "✗ FAIL" };
        let color = if test.passed { "\x1b[32m" } else { "\x1b[31m" };
        println!(
            "│ {:<36} │ {}{:<6}\x1b[0m │ {:>6}ms │",
            test.name, color, status, test.duration_ms
        );

        if test.passed {
            passed += 1;
        } else {
            failed += 1;
            println!("│   └─ {:<56} │", &test.message[..test.message.len().min(56)]);
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
