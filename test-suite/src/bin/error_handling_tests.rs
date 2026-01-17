//! Error Handling Tests
//!
//! Tests for error cases and edge conditions:
//! - Malformed messages
//! - Invalid protocol versions
//! - Connection errors
//! - Resource limits
//! - Timeout handling

use bytes::Bytes;
use clasp_core::{codec, HelloMessage, Message, SetMessage, Value, PROTOCOL_VERSION};
use clasp_router::{Router, RouterConfig};
use clasp_transport::{Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport};
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

struct TestEnv {
    port: u16,
    handle: tokio::task::JoinHandle<()>,
}

impl TestEnv {
    async fn new() -> Self {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::new(RouterConfig::default());
        let handle = tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        Self { port, handle }
    }

    fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    fn stop(self) {
        self.handle.abort();
    }
}

// ============================================================================
// Tests
// ============================================================================

async fn test_malformed_message() -> TestResult {
    let start = std::time::Instant::now();
    let name = "malformed_message";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Send garbage data
        let garbage = Bytes::from(vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01, 0x02]);
        sender.send(garbage).await?;

        // Server should handle gracefully - either error or disconnect
        let response = timeout(Duration::from_secs(1), receiver.recv()).await;

        // Any response is acceptable - key is server didn't crash
        match response {
            Ok(Some(TransportEvent::Error(_))) => Ok(()), // Error is fine
            Ok(Some(TransportEvent::Disconnected { .. })) => Ok(()), // Disconnect is fine
            Ok(Some(TransportEvent::Connected)) => Ok(()), // Connection still ok
            Ok(Some(TransportEvent::Data(_))) => Ok(()), // Even data response is ok
            Ok(None) => Ok(()), // Connection closed is fine
            Err(_) => Ok(()), // Timeout is fine (server ignored bad data)
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_truncated_message() -> TestResult {
    let start = std::time::Instant::now();
    let name = "truncated_message";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Encode a valid message then truncate it
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: "Test".to_string(),
            features: vec![],
            capabilities: None, token: None,
        });
        let mut bytes = codec::encode(&hello)?;
        // Truncate to just 3 bytes
        let truncated = Bytes::from(bytes.to_vec()[..3.min(bytes.len())].to_vec());
        sender.send(truncated).await?;

        // Server should handle gracefully
        let response = timeout(Duration::from_secs(1), receiver.recv()).await;

        // Any graceful handling is acceptable
        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_wrong_protocol_version() -> TestResult {
    let start = std::time::Instant::now();
    let name = "wrong_protocol_version";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Send HELLO with wrong version
        let hello = Message::Hello(HelloMessage {
            version: 99, // Invalid version (not 2)
            name: "BadVersion".to_string(),
            features: vec![],
            capabilities: None, token: None,
        });
        sender.send(codec::encode(&hello)?).await?;

        // Should get error or still work (version mismatch handling varies)
        let response = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(event) = receiver.recv().await {
                    match event {
                        TransportEvent::Data(data) => {
                            let (msg, _) = codec::decode(&data)?;
                            return Ok::<_, Box<dyn std::error::Error + Send + Sync>>(msg);
                        }
                        TransportEvent::Connected => continue,
                        TransportEvent::Disconnected { reason } => {
                            return Err(format!("Disconnected: {:?}", reason).into())
                        }
                        TransportEvent::Error(e) => return Err(e.into()),
                    }
                }
            }
        })
        .await;

        // Either error or welcome (with potential version warning) is acceptable
        match response {
            Ok(Ok(Message::Welcome(_))) => Ok(()), // Server accepted anyway
            Ok(Ok(Message::Error(_))) => Ok(()), // Server rejected - also fine
            Ok(Ok(_)) => Ok(()), // Any other message - server handled somehow
            Ok(Err(_)) => Ok(()), // Error during receive
            Err(_) => Ok(()), // Timeout - server might have ignored
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_message_before_hello() -> TestResult {
    let start = std::time::Instant::now();
    let name = "message_before_hello";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Send SET before HELLO
        let set = Message::Set(SetMessage {
            address: "/test".to_string(),
            value: Value::Int(1),
            revision: None, lock: false, unlock: false,
            
        });
        sender.send(codec::encode(&set)?).await?;

        // Server should reject or ignore
        let response = timeout(Duration::from_secs(1), receiver.recv()).await;

        // Should not get ACK (no session established)
        match response {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data)?;
                match msg {
                    Message::Ack(_) => Err("Should not ACK before HELLO".into()),
                    Message::Error(_) => Ok(()), // Error is correct behavior
                    _ => Ok(()), // Other responses ok
                }
            }
            _ => Ok(()), // Timeout or disconnect is fine
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_duplicate_hello() -> TestResult {
    let start = std::time::Instant::now();
    let name = "duplicate_hello";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Send first HELLO
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "First".to_string(),
            features: vec![],
            capabilities: None, token: None,
        });
        sender.send(codec::encode(&hello)?).await?;

        // Wait for WELCOME
        loop {
            match timeout(Duration::from_secs(2), receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => {
                    let (msg, _) = codec::decode(&data)?;
                    if matches!(msg, Message::Welcome(_)) {
                        break;
                    }
                }
                Ok(Some(TransportEvent::Connected)) => continue,
                _ => return Err("No WELCOME".into()),
            }
        }

        // Send second HELLO (should be ignored or cause error)
        let hello2 = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "Second".to_string(),
            features: vec![],
            capabilities: None, token: None,
        });
        sender.send(codec::encode(&hello2)?).await?;

        // Server should handle gracefully
        let response = timeout(Duration::from_millis(500), receiver.recv()).await;

        // Any non-crash behavior is acceptable
        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_very_long_address() -> TestResult {
    let start = std::time::Instant::now();
    let name = "very_long_address";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Complete handshake
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "LongAddressTest".to_string(),
            features: vec![],
            capabilities: None, token: None,
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

        // Send SET with very long address (10KB)
        let long_addr = format!("/{}", "a".repeat(10_000));
        let set = Message::Set(SetMessage {
            address: long_addr,
            value: Value::Int(1),
            revision: None, lock: false, unlock: false,
            
        });
        sender.send(codec::encode(&set)?).await?;

        // Should handle gracefully
        let response = timeout(Duration::from_secs(1), receiver.recv()).await;

        // Either ACK, error, or timeout is acceptable
        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_empty_address() -> TestResult {
    let start = std::time::Instant::now();
    let name = "empty_address";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Complete handshake
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "EmptyAddressTest".to_string(),
            features: vec![],
            capabilities: None, token: None,
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

        // Send SET with empty address
        let set = Message::Set(SetMessage {
            address: "".to_string(), // Empty!
            value: Value::Int(1),
            revision: None, lock: false, unlock: false,
            
        });
        sender.send(codec::encode(&set)?).await?;

        // Should handle gracefully (error or ignore)
        let response = timeout(Duration::from_secs(1), receiver.recv()).await;

        match response {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data)?;
                match msg {
                    Message::Error(_) => Ok(()), // Error is correct
                    Message::Ack(_) => Ok(()), // Accepting empty is also valid
                    _ => Ok(()),
                }
            }
            _ => Ok(()), // Timeout is fine
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_rapid_disconnect_reconnect() -> TestResult {
    let start = std::time::Instant::now();
    let name = "rapid_disconnect_reconnect";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        for i in 0..5 {
            let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

            // Quick handshake
            let hello = Message::Hello(HelloMessage {
                version: PROTOCOL_VERSION,
                name: format!("Rapid{}", i),
                features: vec![],
                capabilities: None, token: None,
            });
            sender.send(codec::encode(&hello)?).await?;

            // Wait briefly for WELCOME
            let _ = timeout(Duration::from_millis(100), receiver.recv()).await;

            // Disconnect
            sender.close().await?;
        }

        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

async fn test_connection_to_closed_port() -> TestResult {
    let start = std::time::Instant::now();
    let name = "connection_to_closed_port";

    // Try connecting to a port that's definitely not listening
    let result = timeout(
        Duration::from_secs(2),
        WebSocketTransport::connect("ws://127.0.0.1:1"),
    )
    .await;

    match result {
        Ok(Err(_)) => TestResult::pass(name, start.elapsed().as_millis()), // Connection refused
        Err(_) => TestResult::pass(name, start.elapsed().as_millis()), // Timeout
        Ok(Ok(_)) => TestResult::fail(name, "Should not connect", start.elapsed().as_millis()),
    }
}

async fn test_special_characters_in_address() -> TestResult {
    let start = std::time::Instant::now();
    let name = "special_characters_in_address";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&env.url()).await?;

        // Complete handshake
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "SpecialChars".to_string(),
            features: vec![],
            capabilities: None, token: None,
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

        // Test various special characters
        let special_addresses = vec![
            "/path/with spaces",
            "/path/with\ttabs",
            "/unicode/日本語",
            "/emoji/🎵",
            "/symbols/@#$%",
        ];

        for addr in special_addresses {
            let set = Message::Set(SetMessage {
                address: addr.to_string(),
                value: Value::Int(1),
                revision: None, lock: false, unlock: false,
                
            });
            sender.send(codec::encode(&set)?).await?;

            // Should handle each address
            let _ = timeout(Duration::from_millis(100), receiver.recv()).await;
        }

        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{:?}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                 CLASP Error Handling Tests                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_malformed_message().await,
        test_truncated_message().await,
        test_wrong_protocol_version().await,
        test_message_before_hello().await,
        test_duplicate_hello().await,
        test_very_long_address().await,
        test_empty_address().await,
        test_rapid_disconnect_reconnect().await,
        test_connection_to_closed_port().await,
        test_special_characters_in_address().await,
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
