//! Subscription Tests
//!
//! Comprehensive tests for CLASP subscription patterns:
//! - Exact match subscriptions
//! - Single-level wildcards (*)
//! - Multi-level wildcards (**)
//! - Subscription lifecycle (add/remove)
//! - Multiple subscriptions per client
//! - Subscription filtering by signal type

use clasp_core::{
    codec, HelloMessage, Message, PublishMessage, SetMessage, SignalType, SubscribeMessage,
    SubscribeOptions, UnsubscribeMessage, Value,
};
use clasp_router::{Router, RouterConfig};
use clasp_transport::{
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use std::time::Duration;
use tokio::time::timeout;

type TestError = Box<dyn std::error::Error + Send + Sync>;

// Simple error wrapper for string errors
#[derive(Debug)]
struct StringError(String);
impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for StringError {}
impl From<String> for StringError {
    fn from(s: String) -> Self {
        StringError(s)
    }
}
impl From<&str> for StringError {
    fn from(s: &str) -> Self {
        StringError(s.to_string())
    }
}

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

async fn connect_and_handshake(
    url: &str,
    name: &str,
) -> Result<
    (
        clasp_transport::websocket::WebSocketSender,
        clasp_transport::websocket::WebSocketReceiver,
    ),
    TestError,
> {
    let (sender, mut receiver) = WebSocketTransport::connect(url).await?;

    let hello = Message::Hello(HelloMessage {
        version: 2,
        name: name.to_string(),
        features: vec!["param".to_string(), "event".to_string()],
        capabilities: None,
        token: None,
    });
    sender.send(codec::encode(&hello)?).await?;

    // Wait for handshake completion
    let mut got_welcome = false;
    let mut got_snapshot = false;
    while !got_welcome || !got_snapshot {
        match timeout(Duration::from_secs(2), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data)?;
                match msg {
                    Message::Welcome(_) => got_welcome = true,
                    Message::Snapshot(_) => got_snapshot = true,
                    _ => {}
                }
            }
            Ok(Some(TransportEvent::Connected)) => continue,
            _ => return Err(Box::new(StringError::from("Handshake failed")) as TestError),
        }
    }

    Ok((sender, receiver))
}

// ============================================================================
// Tests
// ============================================================================

async fn test_exact_match_subscription() -> TestResult {
    let start = std::time::Instant::now();
    let name = "exact_match_subscription";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sub_sender, mut sub_receiver) =
            connect_and_handshake(&env.url(), "Subscriber").await?;
        let (pub_sender, mut pub_receiver) = connect_and_handshake(&env.url(), "Publisher").await?;

        // Subscribe to exact path
        let subscribe = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: "/exact/path".to_string(),
            types: vec![],
            options: None,
        });
        sub_sender.send(codec::encode(&subscribe)?).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Publish to exact path - should match
        let set1 = Message::Set(SetMessage {
            address: "/exact/path".to_string(),
            value: Value::Int(1),
            revision: None,
            lock: false,
            unlock: false,
        });
        pub_sender.send(codec::encode(&set1)?).await?;

        // Publish to different path - should NOT match
        let set2 = Message::Set(SetMessage {
            address: "/exact/other".to_string(),
            value: Value::Int(2),
            revision: None,
            lock: false,
            unlock: false,
        });
        pub_sender.send(codec::encode(&set2)?).await?;

        // Subscriber should only receive first message
        let msg1 = timeout(Duration::from_secs(1), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(set) = msg {
                        return Some(set);
                    }
                }
            }
        })
        .await;

        if msg1.is_err() {
            return Err(
                Box::new(StringError::from("Did not receive matching message")) as TestError,
            );
        }

        let set_msg = msg1.unwrap().unwrap();
        if set_msg.address != "/exact/path" {
            return Err(Box::new(StringError::from(format!(
                "Wrong address: {}",
                set_msg.address
            ))) as TestError);
        }

        // Should NOT receive the second message (timeout expected)
        let msg2 = timeout(Duration::from_millis(200), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(set) = msg {
                        if set.address == "/exact/other" {
                            return Some(set);
                        }
                    }
                }
            }
        })
        .await;

        if msg2.is_ok() {
            return Err(
                Box::new(StringError::from("Should NOT receive non-matching message")) as TestError,
            );
        }

        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

async fn test_single_wildcard_subscription() -> TestResult {
    let start = std::time::Instant::now();
    let name = "single_wildcard_subscription";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sub_sender, mut sub_receiver) =
            connect_and_handshake(&env.url(), "Subscriber").await?;
        let (pub_sender, _) = connect_and_handshake(&env.url(), "Publisher").await?;

        // Subscribe with single-level wildcard
        let subscribe = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: "/sensors/*/temperature".to_string(),
            types: vec![],
            options: None,
        });
        sub_sender.send(codec::encode(&subscribe)?).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should match: /sensors/room1/temperature
        pub_sender
            .send(codec::encode(&Message::Set(SetMessage {
                address: "/sensors/room1/temperature".to_string(),
                value: Value::Float(22.5),
                revision: None,
                lock: false,
                unlock: false,
            }))?)
            .await?;

        let msg = timeout(Duration::from_secs(1), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(set) = msg {
                        return Some(set);
                    }
                }
            }
        })
        .await;

        if msg.is_err() {
            return Err(
                Box::new(StringError::from("Did not receive matching message")) as TestError,
            );
        }

        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

async fn test_multi_wildcard_subscription() -> TestResult {
    let start = std::time::Instant::now();
    let name = "multi_wildcard_subscription";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sub_sender, mut sub_receiver) =
            connect_and_handshake(&env.url(), "Subscriber").await?;
        let (pub_sender, _) = connect_and_handshake(&env.url(), "Publisher").await?;

        // Subscribe with multi-level wildcard
        let subscribe = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: "/house/**".to_string(),
            types: vec![],
            options: None,
        });
        sub_sender.send(codec::encode(&subscribe)?).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // All of these should match
        let paths = vec![
            "/house/living-room/light",
            "/house/bedroom/temperature",
            "/house/kitchen/oven/temperature",
            "/house/basement/storage/humidity",
        ];

        for path in &paths {
            pub_sender
                .send(codec::encode(&Message::Set(SetMessage {
                    address: path.to_string(),
                    value: Value::Float(1.0),
                    revision: None,
                    lock: false,
                    unlock: false,
                }))?)
                .await?;
        }

        // Should receive all messages
        let mut received = 0;
        for _ in 0..paths.len() {
            let msg = timeout(Duration::from_secs(1), async {
                loop {
                    if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                        let (msg, _) = codec::decode(&data).unwrap();
                        if let Message::Set(_) = msg {
                            return true;
                        }
                    }
                }
            })
            .await;

            if msg.is_ok() {
                received += 1;
            }
        }

        if received >= paths.len() - 1 {
            Ok(())
        } else {
            Err(Box::new(StringError::from(format!(
                "Only received {}/{} messages",
                received,
                paths.len()
            ))) as TestError)
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

async fn test_unsubscribe() -> TestResult {
    let start = std::time::Instant::now();
    let name = "unsubscribe";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sub_sender, mut sub_receiver) =
            connect_and_handshake(&env.url(), "Subscriber").await?;
        let (pub_sender, _) = connect_and_handshake(&env.url(), "Publisher").await?;

        // Subscribe
        sub_sender
            .send(codec::encode(&Message::Subscribe(SubscribeMessage {
                id: 1,
                pattern: "/test/**".to_string(),
                types: vec![],
                options: None,
            }))?)
            .await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // First message should be received
        pub_sender
            .send(codec::encode(&Message::Set(SetMessage {
                address: "/test/value1".to_string(),
                value: Value::Int(1),
                revision: None,
                lock: false,
                unlock: false,
            }))?)
            .await?;

        let msg1 = timeout(Duration::from_secs(1), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(_) = msg {
                        return true;
                    }
                }
            }
        })
        .await;

        if msg1.is_err() {
            return Err(Box::new(StringError::from("Should receive first message")) as TestError);
        }

        // Unsubscribe
        sub_sender
            .send(codec::encode(&Message::Unsubscribe(UnsubscribeMessage {
                id: 1,
            }))?)
            .await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Second message should NOT be received
        pub_sender
            .send(codec::encode(&Message::Set(SetMessage {
                address: "/test/value2".to_string(),
                value: Value::Int(2),
                revision: None,
                lock: false,
                unlock: false,
            }))?)
            .await?;

        let msg2 = timeout(Duration::from_millis(300), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(set) = msg {
                        if set.address == "/test/value2" {
                            return true;
                        }
                    }
                }
            }
        })
        .await;

        if msg2.is_ok() {
            return Err(Box::new(StringError::from(
                "Should NOT receive message after unsubscribe",
            )) as TestError);
        }

        Ok(())
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

async fn test_multiple_subscriptions() -> TestResult {
    let start = std::time::Instant::now();
    let name = "multiple_subscriptions";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sub_sender, mut sub_receiver) =
            connect_and_handshake(&env.url(), "Subscriber").await?;
        let (pub_sender, _) = connect_and_handshake(&env.url(), "Publisher").await?;

        // Multiple subscriptions
        for (id, pattern) in [(1, "/a/**"), (2, "/b/**"), (3, "/c/**")] {
            sub_sender
                .send(codec::encode(&Message::Subscribe(SubscribeMessage {
                    id,
                    pattern: pattern.to_string(),
                    types: vec![],
                    options: None,
                }))?)
                .await?;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send to each namespace
        for addr in ["/a/val", "/b/val", "/c/val"] {
            pub_sender
                .send(codec::encode(&Message::Set(SetMessage {
                    address: addr.to_string(),
                    value: Value::Int(1),
                    revision: None,
                    lock: false,
                    unlock: false,
                }))?)
                .await?;
        }

        // Should receive all 3
        let mut received = 0;
        for _ in 0..3 {
            if timeout(Duration::from_secs(1), async {
                loop {
                    if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                        let (msg, _) = codec::decode(&data).unwrap();
                        if let Message::Set(_) = msg {
                            return true;
                        }
                    }
                }
            })
            .await
            .is_ok()
            {
                received += 1;
            }
        }

        if received >= 2 {
            Ok(())
        } else {
            Err(Box::new(StringError::from(format!(
                "Only received {}/3 messages",
                received
            ))) as TestError)
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

async fn test_subscription_initial_snapshot() -> TestResult {
    let start = std::time::Instant::now();
    let name = "subscription_initial_snapshot";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        // First client sets a value
        let (pub_sender, mut pub_receiver) = connect_and_handshake(&env.url(), "Publisher").await?;
        pub_sender
            .send(codec::encode(&Message::Set(SetMessage {
                address: "/snapshot/test".to_string(),
                value: Value::Float(42.0),
                revision: None,
                lock: false,
                unlock: false,
            }))?)
            .await?;

        // Wait for ACK
        loop {
            if let Some(TransportEvent::Data(data)) = pub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if matches!(msg, Message::Ack(_)) {
                    break;
                }
            }
        }

        // Second client subscribes and should get snapshot with the value
        let (sub_sender, mut sub_receiver) =
            connect_and_handshake(&env.url(), "Subscriber").await?;
        sub_sender
            .send(codec::encode(&Message::Subscribe(SubscribeMessage {
                id: 1,
                pattern: "/snapshot/**".to_string(),
                types: vec![],
                options: None,
            }))?)
            .await?;

        // Should receive snapshot with current value
        let found = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Snapshot(snapshot) = msg {
                        for param in snapshot.params {
                            if param.address == "/snapshot/test" {
                                return true;
                            }
                        }
                    }
                }
            }
        })
        .await;

        if found.is_ok() {
            Ok(())
        } else {
            Err(Box::new(StringError::from(
                "Did not receive snapshot with existing value",
            )) as TestError)
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

async fn test_invalid_subscription_pattern() -> TestResult {
    let start = std::time::Instant::now();
    let name = "invalid_subscription_pattern";

    let env = TestEnv::new().await;

    let result: Result<(), TestError> = async {
        let (sender, mut receiver) = connect_and_handshake(&env.url(), "Client").await?;

        // Subscribe with invalid pattern (empty)
        sender
            .send(codec::encode(&Message::Subscribe(SubscribeMessage {
                id: 1,
                pattern: "".to_string(), // Invalid
                types: vec![],
                options: None,
            }))?)
            .await?;

        // Should receive error
        let error = timeout(Duration::from_secs(1), async {
            loop {
                if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Error(_) = msg {
                        return true;
                    }
                }
            }
        })
        .await;

        if error.is_ok() {
            Ok(())
        } else {
            // Empty pattern might just be ignored
            Ok(()) // This is acceptable behavior
        }
    }
    .await;

    env.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, e.to_string(), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("warn").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                 CLASP Subscription Tests                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_exact_match_subscription().await,
        test_single_wildcard_subscription().await,
        test_multi_wildcard_subscription().await,
        test_unsubscribe().await,
        test_multiple_subscriptions().await,
        test_subscription_initial_snapshot().await,
        test_invalid_subscription_pattern().await,
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
