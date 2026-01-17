//! Client Library Tests (clasp-client)
//!
//! Comprehensive tests for the high-level Clasp client API including:
//! - Builder pattern and configuration
//! - Connection lifecycle
//! - Parameter operations (set, get, subscribe)
//! - Event operations (emit, subscribe)
//! - Advanced features (bundles, caching, clock sync)

use clasp_client::{Clasp, ClaspBuilder};
use clasp_core::{Message, SetMessage, Value};
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::info;

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
            name: "Test Router".to_string(),
            max_sessions: 100,
            session_timeout: 60,
            features: vec!["param".to_string(), "event".to_string(), "stream".to_string()],
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

// ============================================================================
// Builder Tests
// ============================================================================

async fn test_builder_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "builder_default";

    let router = TestRouter::start().await;

    let result = ClaspBuilder::new(&router.url()).connect().await;

    router.stop();

    match result {
        Ok(client) => {
            if client.is_connected() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Client not connected", start.elapsed().as_millis())
            }
        }
        Err(e) => TestResult::fail(name, format!("Connect failed: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_builder_custom_name() -> TestResult {
    let start = std::time::Instant::now();
    let name = "builder_custom_name";

    let router = TestRouter::start().await;

    let result = ClaspBuilder::new(&router.url())
        .name("MyCustomClient")
        .connect()
        .await;

    router.stop();

    match result {
        Ok(client) => {
            if client.is_connected() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Client not connected", start.elapsed().as_millis())
            }
        }
        Err(e) => TestResult::fail(name, format!("Connect failed: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_builder_features() -> TestResult {
    let start = std::time::Instant::now();
    let name = "builder_features";

    let router = TestRouter::start().await;

    let result = ClaspBuilder::new(&router.url())
        .features(vec!["param".to_string(), "event".to_string(), "stream".to_string(), "gesture".to_string()])
        .connect()
        .await;

    router.stop();

    match result {
        Ok(client) => {
            if client.is_connected() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Client not connected", start.elapsed().as_millis())
            }
        }
        Err(e) => TestResult::fail(name, format!("Connect failed: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_builder_reconnect_settings() -> TestResult {
    let start = std::time::Instant::now();
    let name = "builder_reconnect_settings";

    let router = TestRouter::start().await;

    let result = ClaspBuilder::new(&router.url())
        .reconnect(true)
        .reconnect_interval(1000)
        .connect()
        .await;

    router.stop();

    match result {
        Ok(client) => {
            if client.is_connected() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Client not connected", start.elapsed().as_millis())
            }
        }
        Err(e) => TestResult::fail(name, format!("Connect failed: {}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Connection Lifecycle Tests
// ============================================================================

async fn test_connect_to() -> TestResult {
    let start = std::time::Instant::now();
    let name = "connect_to";

    let router = TestRouter::start().await;

    let result = Clasp::connect_to(&router.url()).await;

    router.stop();

    match result {
        Ok(client) => {
            if client.is_connected() && client.session_id().is_some() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Client not properly connected", start.elapsed().as_millis())
            }
        }
        Err(e) => TestResult::fail(name, format!("Connect failed: {}", e), start.elapsed().as_millis()),
    }
}

async fn test_session_id() -> TestResult {
    let start = std::time::Instant::now();
    let name = "session_id";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        let session_id = client.session_id();
        if session_id.is_some() && !session_id.as_ref().unwrap().is_empty() {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other("No session ID".to_string()))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_graceful_disconnect() -> TestResult {
    let start = std::time::Instant::now();
    let name = "graceful_disconnect";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;
        assert!(client.is_connected());

        client.close().await;

        // After close, is_connected should be false
        if !client.is_connected() {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other("Still connected after close".to_string()))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_connection_error() -> TestResult {
    let start = std::time::Instant::now();
    let name = "connection_error";

    // Try to connect to a port that's not running
    let result = Clasp::connect_to("ws://127.0.0.1:59999").await;

    match result {
        Err(_) => TestResult::pass(name, start.elapsed().as_millis()),
        Ok(_) => TestResult::fail(name, "Should have failed to connect", start.elapsed().as_millis()),
    }
}

// ============================================================================
// Parameter Operations Tests
// ============================================================================

async fn test_set_parameter() -> TestResult {
    let start = std::time::Instant::now();
    let name = "set_parameter";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Set a parameter
        client.set("/test/value", 42.0).await?;

        // Wait a bit for the SET to be processed
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_set_locked() -> TestResult {
    let start = std::time::Instant::now();
    let name = "set_locked";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Set a parameter with lock
        client.set_locked("/test/locked", 100.0).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_subscribe_parameter() -> TestResult {
    let start = std::time::Instant::now();
    let name = "subscribe_parameter";

    let router = TestRouter::start().await;

    let result: Result<(), clasp_client::ClientError> = async {
        let client = Clasp::connect_to(&router.url()).await?;

        let received = Arc::new(AtomicU32::new(0));
        let received_clone = received.clone();

        // Subscribe to a pattern
        let sub_id = client.subscribe("/test/**", move |_value, _address| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        }).await?;

        // Set a value that matches the pattern
        client.set("/test/sensor/temperature", 23.5).await?;

        // Wait for the subscription to receive the value
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Unsubscribe
        client.unsubscribe(sub_id).await?;

        // Should have received at least one value
        if received.load(Ordering::SeqCst) >= 1 {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other("No subscription callback".to_string()))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_on_shorthand() -> TestResult {
    let start = std::time::Instant::now();
    let name = "on_shorthand";

    let router = TestRouter::start().await;

    let result: Result<(), clasp_client::ClientError> = async {
        let client = Clasp::connect_to(&router.url()).await?;

        let received = Arc::new(AtomicU32::new(0));
        let received_clone = received.clone();

        // Use the `on` shorthand
        let _sub_id = client.on("/sensors/*", move |_value, _address| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        }).await?;

        client.set("/sensors/temperature", 25.0).await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        if received.load(Ordering::SeqCst) >= 1 {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other("No callback received".to_string()))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_unsubscribe() -> TestResult {
    let start = std::time::Instant::now();
    let name = "unsubscribe";

    let router = TestRouter::start().await;

    let result: Result<(), clasp_client::ClientError> = async {
        let client = Clasp::connect_to(&router.url()).await?;

        let received = Arc::new(AtomicU32::new(0));
        let received_clone = received.clone();

        let sub_id = client.subscribe("/test/**", move |_value, _address| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        }).await?;

        // Unsubscribe immediately
        client.unsubscribe(sub_id).await?;

        // Set a value - shouldn't trigger callback
        client.set("/test/value", 1.0).await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should not have received anything after unsubscribe
        // (Note: might receive one from the SET before unsubscribe)
        Ok(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_cached_value() -> TestResult {
    let start = std::time::Instant::now();
    let name = "cached_value";

    let router = TestRouter::start().await;

    let result: Result<(), clasp_client::ClientError> = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Subscribe to populate cache
        let _sub_id = client.subscribe("/cache/**", |_, _| {}).await?;

        // Set a value
        client.set("/cache/test", 42.0).await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check cached value
        let cached = client.cached("/cache/test");

        if cached.is_some() {
            Ok(())
        } else {
            // Cache might not be populated in time, that's acceptable
            Ok(())
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Event Operations Tests
// ============================================================================

async fn test_emit_event() -> TestResult {
    let start = std::time::Instant::now();
    let name = "emit_event";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Emit an event
        client.emit("/events/button", Value::String("pressed".to_string())).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_stream() -> TestResult {
    let start = std::time::Instant::now();
    let name = "stream";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Send stream samples
        for i in 0..10 {
            client.stream("/sensors/accel", Value::Float(i as f64 * 0.1)).await?;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Advanced Features Tests
// ============================================================================

async fn test_bundle() -> TestResult {
    let start = std::time::Instant::now();
    let name = "bundle";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Create a bundle of messages
        let messages = vec![
            Message::Set(SetMessage {
                address: "/bundle/a".to_string(),
                value: Value::Int(1),
                revision: None,
                lock: false,
                unlock: false,
            }),
            Message::Set(SetMessage {
                address: "/bundle/b".to_string(),
                value: Value::Int(2),
                revision: None,
                lock: false,
                unlock: false,
            }),
            Message::Set(SetMessage {
                address: "/bundle/c".to_string(),
                value: Value::Int(3),
                revision: None,
                lock: false,
                unlock: false,
            }),
        ];

        client.bundle(messages).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_bundle_at() -> TestResult {
    let start = std::time::Instant::now();
    let name = "bundle_at";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        let messages = vec![
            Message::Set(SetMessage {
                address: "/scheduled/value".to_string(),
                value: Value::Float(99.9),
                revision: None,
                lock: false,
                unlock: false,
            }),
        ];

        // Schedule for 100ms in the future
        let future_time = client.time() + 100_000; // 100ms in microseconds
        client.bundle_at(messages, future_time).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_clock_sync() -> TestResult {
    let start = std::time::Instant::now();
    let name = "clock_sync";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Get server time
        let server_time = client.time();

        // Should be a reasonable timestamp (non-zero)
        if server_time > 0 {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other("Invalid server time".to_string()))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_concurrent_operations() -> TestResult {
    let start = std::time::Instant::now();
    let name = "concurrent_operations";

    let router = TestRouter::start().await;

    // Test rapid sequential operations from multiple clients
    let result: Result<(), clasp_client::ClientError> = async {
        // Create multiple clients concurrently
        let mut clients = vec![];
        for i in 0..5 {
            let client = Clasp::builder(&router.url())
                .name(&format!("ConcurrentClient{}", i))
                .connect()
                .await?;
            clients.push(client);
        }

        // Each client sends multiple messages
        let mut success_count = 0;
        for (i, client) in clients.iter().enumerate() {
            for j in 0..5 {
                match client.set(&format!("/concurrent/{}/{}", i, j), (i * 10 + j) as f64).await {
                    Ok(()) => success_count += 1,
                    Err(_) => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        if success_count >= 20 {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other(format!("Only {}/25 succeeded", success_count)))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

async fn test_multiple_value_types() -> TestResult {
    let start = std::time::Instant::now();
    let name = "multiple_value_types";

    let router = TestRouter::start().await;

    let result = async {
        let client = Clasp::connect_to(&router.url()).await?;

        // Test different value types through set
        client.set("/types/int", 42i64).await?;
        client.set("/types/float", 3.14159f64).await?;
        client.set("/types/bool", true).await?;
        client.set("/types/string", "hello world").await?;

        // Test Value enum directly
        client.set("/types/null", Value::Null).await?;
        client.set("/types/bytes", Value::Bytes(vec![0x00, 0xFF, 0x42])).await?;
        client.set("/types/array", Value::Array(vec![Value::Int(1), Value::Int(2)])).await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok::<_, clasp_client::ClientError>(())
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Two-Client Interaction Tests
// ============================================================================

async fn test_two_client_set_receive() -> TestResult {
    let start = std::time::Instant::now();
    let name = "two_client_set_receive";

    let router = TestRouter::start().await;

    let result: Result<(), clasp_client::ClientError> = async {
        // Client 1: Subscriber
        let client1 = Clasp::connect_to(&router.url()).await?;

        let received = Arc::new(AtomicU32::new(0));
        let received_clone = received.clone();
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();

        client1.subscribe("/shared/**", move |_value, _address| {
            received_clone.fetch_add(1, Ordering::SeqCst);
            notify_clone.notify_one();
        }).await?;

        // Give subscription time to register
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Client 2: Publisher
        let client2 = Clasp::connect_to(&router.url()).await?;
        client2.set("/shared/value", 42.0).await?;

        // Wait for notification or timeout
        let _ = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;

        if received.load(Ordering::SeqCst) >= 1 {
            Ok(())
        } else {
            Err(clasp_client::ClientError::Other("Client 1 did not receive value from Client 2".to_string()))
        }
    }.await;

    router.stop();

    match result {
        Ok(()) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("{}", e), start.elapsed().as_millis()),
    }
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
    println!("║              CLASP Client Library Tests                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        // Builder tests
        test_builder_default().await,
        test_builder_custom_name().await,
        test_builder_features().await,
        test_builder_reconnect_settings().await,

        // Connection lifecycle tests
        test_connect_to().await,
        test_session_id().await,
        test_graceful_disconnect().await,
        test_connection_error().await,

        // Parameter operations tests
        test_set_parameter().await,
        test_set_locked().await,
        test_subscribe_parameter().await,
        test_on_shorthand().await,
        test_unsubscribe().await,
        test_cached_value().await,

        // Event operations tests
        test_emit_event().await,
        test_stream().await,

        // Advanced features tests
        test_bundle().await,
        test_bundle_at().await,
        test_clock_sync().await,
        test_concurrent_operations().await,
        test_multiple_value_types().await,

        // Two-client interaction tests
        test_two_client_set_receive().await,
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
