//! Relay Server End-to-End Tests
//!
//! Comprehensive E2E tests for the CLASP relay server including:
//! - Server startup and shutdown
//! - Client connections
//! - Message routing between clients
//! - State synchronization
//! - Subscription patterns
//! - Multi-client scenarios
//! - Connection resilience

use clasp_core::{
    codec, HelloMessage, Message, PublishMessage, SecurityMode, SetMessage, SubscribeMessage, Value,
};
use clasp_router::{Router, RouterConfig};
use clasp_transport::{
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

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
// Test Utilities
// ============================================================================

async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

struct TestRouter {
    port: u16,
    handle: tokio::task::JoinHandle<()>,
}

impl TestRouter {
    async fn start() -> Self {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::new(RouterConfig {
            name: "Test Relay".to_string(),
            max_sessions: 100,
            session_timeout: 60,
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
            ],
            security_mode: SecurityMode::Open,
            max_subscriptions_per_session: 1000,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        });

        let handle = tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        });

        // Wait for server to start
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

struct TestClient {
    sender: clasp_transport::websocket::WebSocketSender,
    receiver: clasp_transport::websocket::WebSocketReceiver,
    name: String,
}

impl TestClient {
    async fn connect(url: &str, name: &str) -> Result<Self, String> {
        let (sender, receiver) = WebSocketTransport::connect(url)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        Ok(Self {
            sender,
            receiver,
            name: name.to_string(),
        })
    }

    async fn handshake(&mut self) -> Result<(), String> {
        // Send HELLO
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: self.name.clone(),
            features: vec!["param".to_string(), "event".to_string()],
            capabilities: None,
            token: None,
        });

        self.sender
            .send(codec::encode(&hello).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| format!("Send failed: {}", e))?;

        // Wait for WELCOME
        let mut got_welcome = false;
        let mut got_snapshot = false;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

        while !got_welcome || !got_snapshot {
            if tokio::time::Instant::now() > deadline {
                return Err("Handshake timeout".to_string());
            }

            match timeout(Duration::from_secs(1), self.receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => {
                    let (msg, _) = codec::decode(&data).map_err(|e| e.to_string())?;
                    match msg {
                        Message::Welcome(_) => got_welcome = true,
                        Message::Snapshot(_) => got_snapshot = true,
                        _ => {}
                    }
                }
                Ok(Some(TransportEvent::Connected)) => continue,
                Ok(Some(TransportEvent::Disconnected { reason })) => {
                    return Err(format!("Disconnected during handshake: {:?}", reason));
                }
                Ok(Some(TransportEvent::Error(e))) => {
                    return Err(format!("Error during handshake: {}", e));
                }
                Ok(None) => return Err("Connection closed".to_string()),
                Err(_) => continue, // Timeout, keep waiting
            }
        }

        Ok(())
    }

    async fn subscribe(&mut self, pattern: &str, id: u32) -> Result<(), String> {
        let subscribe = Message::Subscribe(SubscribeMessage {
            id,
            pattern: pattern.to_string(),
            types: vec![],
            options: None,
        });

        self.sender
            .send(codec::encode(&subscribe).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| format!("Send failed: {}", e))
    }

    async fn set(&mut self, address: &str, value: Value) -> Result<(), String> {
        let set = Message::Set(SetMessage {
            address: address.to_string(),
            value,
            revision: None,
            lock: false,
            unlock: false,
        });

        self.sender
            .send(codec::encode(&set).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| format!("Send failed: {}", e))
    }

    async fn publish(&mut self, address: &str, value: Value) -> Result<(), String> {
        let publish = Message::Publish(PublishMessage {
            address: address.to_string(),
            signal: None,
            value: Some(value),
            payload: None,
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
            timeline: None,
        });

        self.sender
            .send(codec::encode(&publish).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| format!("Send failed: {}", e))
    }

    async fn recv_message(&mut self, timeout_ms: u64) -> Result<Message, String> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("Timeout".to_string());
            }

            match timeout(remaining, self.receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => {
                    let (msg, _) = codec::decode(&data).map_err(|e| e.to_string())?;
                    return Ok(msg);
                }
                Ok(Some(TransportEvent::Connected)) => {
                    // Skip connected events, keep waiting for data
                    continue;
                }
                Ok(Some(TransportEvent::Disconnected { reason })) => {
                    return Err(format!("Disconnected: {:?}", reason));
                }
                Ok(Some(TransportEvent::Error(e))) => return Err(format!("Error: {}", e)),
                Ok(None) => return Err("Connection closed".to_string()),
                Err(_) => return Err("Timeout".to_string()),
            }
        }
    }

    async fn close(self) {
        let _ = self.sender.close().await;
    }
}

// ============================================================================
// Tests
// ============================================================================

async fn test_server_startup() -> TestResult {
    let start = std::time::Instant::now();
    let name = "server_startup";

    let router = TestRouter::start().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    router.stop();

    TestResult::pass(name, start.elapsed().as_millis())
}

async fn test_client_connect() -> TestResult {
    let start = std::time::Instant::now();
    let name = "client_connect";

    let router = TestRouter::start().await;

    match TestClient::connect(&router.url(), "TestClient").await {
        Ok(client) => {
            client.close().await;
            router.stop();
            TestResult::pass(name, start.elapsed().as_millis())
        }
        Err(e) => {
            router.stop();
            TestResult::fail(name, e, start.elapsed().as_millis())
        }
    }
}

async fn test_handshake() -> TestResult {
    let start = std::time::Instant::now();
    let name = "handshake";

    let router = TestRouter::start().await;

    match TestClient::connect(&router.url(), "TestClient").await {
        Ok(mut client) => match client.handshake().await {
            Ok(()) => {
                client.close().await;
                router.stop();
                TestResult::pass(name, start.elapsed().as_millis())
            }
            Err(e) => {
                router.stop();
                TestResult::fail(name, e, start.elapsed().as_millis())
            }
        },
        Err(e) => {
            router.stop();
            TestResult::fail(name, e, start.elapsed().as_millis())
        }
    }
}

async fn test_set_and_ack() -> TestResult {
    let start = std::time::Instant::now();
    let name = "set_and_ack";

    let router = TestRouter::start().await;

    let result = async {
        let mut client = TestClient::connect(&router.url(), "TestClient").await?;
        client.handshake().await?;

        // Send SET
        client.set("/test/value", Value::Float(42.0)).await?;

        // Wait for ACK
        let msg = client.recv_message(2000).await?;
        match msg {
            Message::Ack(ack) => {
                if ack.address == Some("/test/value".to_string()) {
                    Ok(())
                } else {
                    Err(format!("Wrong ACK address: {:?}", ack.address))
                }
            }
            other => Err(format!("Expected ACK, got {:?}", other)),
        }
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_subscription_delivery() -> TestResult {
    let start = std::time::Instant::now();
    let name = "subscription_delivery";

    let router = TestRouter::start().await;

    let result = async {
        // Client 1: Subscriber
        let mut subscriber = TestClient::connect(&router.url(), "Subscriber").await?;
        subscriber.handshake().await?;
        subscriber.subscribe("/sensor/**", 1).await?;

        // Give subscription time to register
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Client 2: Publisher
        let mut publisher = TestClient::connect(&router.url(), "Publisher").await?;
        publisher.handshake().await?;

        // Publish a value
        publisher
            .set("/sensor/temperature", Value::Float(23.5))
            .await?;

        // Subscriber should receive it
        let msg = subscriber.recv_message(2000).await?;
        match msg {
            Message::Set(set) => {
                if set.address == "/sensor/temperature" && set.value == Value::Float(23.5) {
                    Ok(())
                } else {
                    Err(format!("Wrong SET: {:?}", set))
                }
            }
            other => Err(format!("Expected SET, got {:?}", other)),
        }
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_wildcard_subscription() -> TestResult {
    let start = std::time::Instant::now();
    let name = "wildcard_subscription";

    let router = TestRouter::start().await;

    let result = async {
        let mut subscriber = TestClient::connect(&router.url(), "Subscriber").await?;
        subscriber.handshake().await?;
        subscriber.subscribe("/lights/*/brightness", 1).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut publisher = TestClient::connect(&router.url(), "Publisher").await?;
        publisher.handshake().await?;

        // Should match
        publisher
            .set("/lights/living-room/brightness", Value::Float(0.8))
            .await?;

        let msg = subscriber.recv_message(2000).await?;
        match msg {
            Message::Set(set) if set.address == "/lights/living-room/brightness" => Ok(()),
            other => Err(format!("Expected matching SET, got {:?}", other)),
        }
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_multiple_subscribers() -> TestResult {
    let start = std::time::Instant::now();
    let name = "multiple_subscribers";

    let router = TestRouter::start().await;

    let result = async {
        // Create 3 subscribers
        let mut sub1 = TestClient::connect(&router.url(), "Sub1").await?;
        let mut sub2 = TestClient::connect(&router.url(), "Sub2").await?;
        let mut sub3 = TestClient::connect(&router.url(), "Sub3").await?;

        sub1.handshake().await?;
        sub2.handshake().await?;
        sub3.handshake().await?;

        sub1.subscribe("/broadcast/**", 1).await?;
        sub2.subscribe("/broadcast/**", 1).await?;
        sub3.subscribe("/broadcast/**", 1).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Publisher
        let mut publisher = TestClient::connect(&router.url(), "Publisher").await?;
        publisher.handshake().await?;
        publisher
            .set("/broadcast/message", Value::String("hello".to_string()))
            .await?;

        // All should receive
        let r1 = sub1.recv_message(2000).await;
        let r2 = sub2.recv_message(2000).await;
        let r3 = sub3.recv_message(2000).await;

        if r1.is_ok() && r2.is_ok() && r3.is_ok() {
            Ok(())
        } else {
            Err(format!(
                "Not all subscribers received: r1={:?}, r2={:?}, r3={:?}",
                r1.is_ok(),
                r2.is_ok(),
                r3.is_ok()
            ))
        }
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_rapid_messages() -> TestResult {
    let start = std::time::Instant::now();
    let name = "rapid_messages";

    let router = TestRouter::start().await;

    let result = async {
        let mut client = TestClient::connect(&router.url(), "RapidClient").await?;
        client.handshake().await?;

        // Send 100 rapid messages
        for i in 0..100 {
            client.set(&format!("/rapid/{}", i), Value::Int(i)).await?;
        }

        // Should receive 100 ACKs
        let mut ack_count = 0;
        for _ in 0..100 {
            match client.recv_message(1000).await {
                Ok(Message::Ack(_)) => ack_count += 1,
                Ok(Message::Set(_)) => ack_count += 1, // May receive own SETs
                Ok(_) => {}
                Err(_) => break,
            }
        }

        if ack_count >= 50 {
            // At least half should succeed
            Ok(())
        } else {
            Err(format!("Only got {} ACKs out of 100", ack_count))
        }
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_value_types() -> TestResult {
    let start = std::time::Instant::now();
    let name = "value_types";

    let router = TestRouter::start().await;

    let result = async {
        let mut client = TestClient::connect(&router.url(), "TypeTest").await?;
        client.handshake().await?;

        // Test all value types
        let values = vec![
            ("/type/int", Value::Int(42)),
            ("/type/float", Value::Float(3.14159)),
            ("/type/bool", Value::Bool(true)),
            ("/type/string", Value::String("hello world".to_string())),
            ("/type/null", Value::Null),
            ("/type/bytes", Value::Bytes(vec![0x00, 0xFF, 0x42])),
            (
                "/type/array",
                Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            ),
        ];

        for (addr, value) in values {
            client.set(addr, value).await?;
            match client.recv_message(1000).await {
                Ok(Message::Ack(_)) => {}
                other => return Err(format!("Expected ACK for {}, got {:?}", addr, other)),
            }
        }

        Ok(())
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_concurrent_clients() -> TestResult {
    let start = std::time::Instant::now();
    let name = "concurrent_clients";

    let router = TestRouter::start().await;
    let url = router.url();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let url = url.clone();
            tokio::spawn(async move {
                let mut client = TestClient::connect(&url, &format!("Client{}", i)).await?;
                client.handshake().await?;
                client
                    .set(&format!("/client/{}/value", i), Value::Int(i))
                    .await?;
                client.recv_message(2000).await?;
                Ok::<_, String>(())
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles).await;

    router.stop();

    let success_count = results
        .iter()
        .filter(|r| r.as_ref().map(|r| r.is_ok()).unwrap_or(false))
        .count();

    if success_count >= 8 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only {}/10 clients succeeded", success_count),
            start.elapsed().as_millis(),
        )
    }
}

async fn test_state_persistence() -> TestResult {
    let start = std::time::Instant::now();
    let name = "state_persistence";

    let router = TestRouter::start().await;

    let result = async {
        // Client 1: Set a value
        let mut client1 = TestClient::connect(&router.url(), "Client1").await?;
        client1.handshake().await?;
        client1.set("/persistent/value", Value::Float(99.9)).await?;
        client1.recv_message(1000).await?; // ACK
        client1.close().await;

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Client 2: Subscribe and should get the value in snapshot
        let mut client2 = TestClient::connect(&router.url(), "Client2").await?;

        // Send HELLO
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: "Client2".to_string(),
            features: vec!["param".to_string()],
            capabilities: None,
            token: None,
        });
        client2
            .sender
            .send(codec::encode(&hello).unwrap())
            .await
            .map_err(|e| e.to_string())?;

        // Check snapshot contains our value
        let mut found = false;
        for _ in 0..10 {
            match client2.recv_message(1000).await {
                Ok(Message::Snapshot(snapshot)) => {
                    for param in &snapshot.params {
                        if param.address == "/persistent/value" {
                            found = true;
                            break;
                        }
                    }
                    if found {
                        break;
                    }
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        if found {
            Ok(())
        } else {
            Err("Value not found in snapshot".to_string())
        }
    }
    .await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Relay Server E2E Tests                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_server_startup().await,
        test_client_connect().await,
        test_handshake().await,
        test_set_and_ack().await,
        test_subscription_delivery().await,
        test_wildcard_subscription().await,
        test_multiple_subscribers().await,
        test_rapid_messages().await,
        test_value_types().await,
        test_concurrent_clients().await,
        test_state_persistence().await,
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
            println!("│   └─ {:<56} │", test.message);
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
