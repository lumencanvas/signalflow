//! Transport Layer Tests
//!
//! Tests for WebSocket and other transport implementations:
//! - Connection establishment
//! - Message framing
//! - Reconnection
//! - Error handling
//! - Subprotocol negotiation

use clasp_core::{codec, HelloMessage, Message, WS_SUBPROTOCOL};
use clasp_router::{Router, RouterConfig};
use clasp_transport::{
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use std::time::Duration;
use tokio::time::timeout;

type TestError = Box<dyn std::error::Error + Send + Sync>;

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
// Utilities
// ============================================================================

async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

async fn start_router() -> (u16, tokio::task::JoinHandle<()>) {
    let port = find_available_port().await;
    let addr = format!("127.0.0.1:{}", port);

    let router = Router::new(RouterConfig::default());
    let handle = tokio::spawn(async move {
        let _ = router.serve_websocket(&addr).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    (port, handle)
}

// ============================================================================
// Tests
// ============================================================================

async fn test_websocket_connect() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_connect";

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    let result = WebSocketTransport::connect(&url).await;

    handle.abort();

    match result {
        Ok(_) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_websocket_subprotocol() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_subprotocol";

    // Verify subprotocol constant
    if WS_SUBPROTOCOL != "clasp.v2" {
        return TestResult::fail(
            name,
            format!("Wrong subprotocol: {}", WS_SUBPROTOCOL),
            start.elapsed().as_millis(),
        );
    }

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    // Connect should succeed with proper subprotocol negotiation
    let result = WebSocketTransport::connect(&url).await;

    handle.abort();

    match result {
        Ok(_) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_websocket_binary_frames() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_binary_frames";

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&url).await?;

        // Send HELLO as binary frame
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: "BinaryTest".to_string(),
            features: vec![],
            capabilities: None,
            token: None,
        });
        let bytes = codec::encode(&hello)?;
        sender.send(bytes).await?;

        // Should receive binary WELCOME
        match timeout(Duration::from_secs(2), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data)?;
                match msg {
                    Message::Welcome(_) => Ok(()),
                    other => Err(format!("Expected Welcome, got {:?}", other).into()),
                }
            }
            Ok(Some(TransportEvent::Connected)) => {
                // Try again
                match timeout(Duration::from_secs(2), receiver.recv()).await {
                    Ok(Some(TransportEvent::Data(data))) => {
                        let (msg, _) = codec::decode(&data)?;
                        match msg {
                            Message::Welcome(_) => Ok(()),
                            other => Err(format!("Expected Welcome, got {:?}", other).into()),
                        }
                    }
                    _ => Err("No WELCOME received".into()),
                }
            }
            _ => Err("No response".into()),
        }
    }
    .await;

    handle.abort();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_connection_close() -> TestResult {
    let start = std::time::Instant::now();
    let name = "connection_close";

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    let result: Result<(), TestError> = async {
        let (sender, _receiver) = WebSocketTransport::connect(&url).await?;

        // Close connection
        sender.close().await?;

        // Verify closed
        if sender.is_connected() {
            Err("Still connected after close".into())
        } else {
            Ok(())
        }
    }
    .await;

    handle.abort();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_invalid_url() -> TestResult {
    let start = std::time::Instant::now();
    let name = "invalid_url";

    // Try connecting to non-existent server
    let result = timeout(
        Duration::from_secs(2),
        WebSocketTransport::connect("ws://127.0.0.1:1"),
    )
    .await;

    match result {
        Ok(Err(_)) => TestResult::pass(name, start.elapsed().as_millis()), // Connection refused is expected
        Ok(Ok(_)) => TestResult::fail(name, "Should not connect", start.elapsed().as_millis()),
        Err(_) => TestResult::pass(name, start.elapsed().as_millis()), // Timeout is also acceptable
    }
}

async fn test_large_message() -> TestResult {
    let start = std::time::Instant::now();
    let name = "large_message";

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&url).await?;

        // Complete handshake
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: "LargeTest".to_string(),
            features: vec![],
            capabilities: None,
            token: None,
        });
        sender.send(codec::encode(&hello)?).await?;

        // Wait for handshake
        loop {
            match timeout(Duration::from_secs(2), receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => {
                    let (msg, _) = codec::decode(&data)?;
                    if matches!(msg, Message::Snapshot(_)) {
                        break;
                    }
                }
                Ok(Some(TransportEvent::Connected)) => continue,
                _ => break,
            }
        }

        // Send large message (50KB of data)
        let large_data = vec![0u8; 50_000];
        let set = Message::Set(clasp_core::SetMessage {
            address: "/large/data".to_string(),
            value: clasp_core::Value::Bytes(large_data),
            revision: None,
            lock: false,
            unlock: false,
        });
        sender.send(codec::encode(&set)?).await?;

        // Should get ACK
        match timeout(Duration::from_secs(5), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data)?;
                match msg {
                    Message::Ack(_) => Ok(()),
                    other => Err(format!("Expected ACK, got {:?}", other).into()),
                }
            }
            _ => Err("No ACK for large message".into()),
        }
    }
    .await;

    handle.abort();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_rapid_connect_disconnect() -> TestResult {
    let start = std::time::Instant::now();
    let name = "rapid_connect_disconnect";

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    let mut success_count = 0;

    for _ in 0..10 {
        if let Ok((sender, _)) = WebSocketTransport::connect(&url).await {
            let _ = sender.close().await;
            success_count += 1;
        }
    }

    handle.abort();

    if success_count >= 8 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only {}/10 succeeded", success_count),
            start.elapsed().as_millis(),
        )
    }
}

async fn test_concurrent_connections() -> TestResult {
    let start = std::time::Instant::now();
    let name = "concurrent_connections";

    let (port, handle) = start_router().await;
    let url = format!("ws://127.0.0.1:{}", port);

    let handles: Vec<_> = (0..20)
        .map(|_| {
            let url = url.clone();
            tokio::spawn(async move { WebSocketTransport::connect(&url).await.is_ok() })
        })
        .collect();

    let results = futures::future::join_all(handles).await;
    let success_count = results
        .iter()
        .filter(|r| r.as_ref().unwrap_or(&false) == &true)
        .count();

    handle.abort();

    if success_count >= 15 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only {}/20 succeeded", success_count),
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("warn").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                  CLASP Transport Layer Tests                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_websocket_connect().await,
        test_websocket_subprotocol().await,
        test_websocket_binary_frames().await,
        test_connection_close().await,
        test_invalid_url().await,
        test_large_message().await,
        test_rapid_connect_disconnect().await,
        test_concurrent_connections().await,
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
            println!(
                "│   └─ {:<56} │",
                &test.message[..test.message.len().min(56)]
            );
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
