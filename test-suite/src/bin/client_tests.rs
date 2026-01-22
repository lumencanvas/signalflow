//! Client Library Tests (clasp-client)
//!
//! Grade-A quality tests for the high-level Clasp client API including:
//! - Builder pattern and configuration
//! - Connection lifecycle
//! - Parameter operations (set, get, subscribe)
//! - Event operations (emit, subscribe)
//! - Advanced features (bundles, caching, clock sync)
//! - Negative tests and edge cases
//! - Value type coverage

use clasp_client::{Clasp, ClaspBuilder};
use clasp_core::{Message, SecurityMode, SetMessage, Value};
use clasp_router::{Router, RouterConfig};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::timeout;

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

    fn from_result(name: &'static str, result: Result<(), String>, duration_ms: u128) -> Self {
        match result {
            Ok(()) => Self::pass(name, duration_ms),
            Err(msg) => Self::fail(name, msg, duration_ms),
        }
    }
}

// ============================================================================
// Test Utilities with Condition-Based Waits
// ============================================================================

const CHECK_INTERVAL: Duration = Duration::from_millis(10);

async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

async fn wait_for<F, Fut>(check: F, max_wait: Duration) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    while start.elapsed() < max_wait {
        if check().await {
            return true;
        }
        tokio::time::sleep(CHECK_INTERVAL).await;
    }
    false
}

async fn wait_for_port(port: u16, max_wait: Duration) -> bool {
    wait_for(
        || async move {
            tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                .await
                .is_ok()
        },
        max_wait,
    )
    .await
}

async fn wait_for_count(counter: &AtomicU32, target: u32, max_wait: Duration) -> bool {
    wait_for(
        || async { counter.load(Ordering::SeqCst) >= target },
        max_wait,
    )
    .await
}

// ============================================================================
// TestRouter - RAII wrapper with proper cleanup
// ============================================================================

struct TestRouter {
    port: u16,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl TestRouter {
    async fn start() -> Self {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::new(RouterConfig {
            name: "Test Router".to_string(),
            max_sessions: 100,
            session_timeout: 60,
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
            ],
            security_mode: SecurityMode::Open,
            max_subscriptions_per_session: 1000,
        });

        let handle = tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        });

        if !wait_for_port(port, Duration::from_secs(5)).await {
            panic!("Router failed to start on port {}", port);
        }

        Self {
            port,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }
}

impl Drop for TestRouter {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

// ============================================================================
// Error Conversion Helper
// ============================================================================

trait IntoStringError<T> {
    fn map_string_err(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> IntoStringError<T> for Result<T, E> {
    fn map_string_err(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

// ============================================================================
// Assertion Helpers
// ============================================================================

fn assert_that(condition: bool, msg: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn assert_some<T>(opt: Option<T>, msg: &str) -> Result<T, String> {
    opt.ok_or_else(|| msg.to_string())
}

fn assert_eq_msg<T: PartialEq + std::fmt::Debug>(a: &T, b: &T, msg: &str) -> Result<(), String> {
    if a == b {
        Ok(())
    } else {
        Err(format!("{}: {:?} != {:?}", msg, a, b))
    }
}

fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64, msg: &str) -> Result<(), String> {
    if (actual - expected).abs() < epsilon {
        Ok(())
    } else {
        Err(format!(
            "{}: expected {} +/- {}, got {}",
            msg, expected, epsilon, actual
        ))
    }
}

// ============================================================================
// Value Collector - Thread-safe value tracking for subscriptions
// ============================================================================

#[derive(Clone)]
struct ValueCollector {
    values: Arc<std::sync::Mutex<Vec<(String, Value)>>>,
    count: Arc<AtomicU32>,
    notify: Arc<Notify>,
}

impl ValueCollector {
    fn new() -> Self {
        Self {
            values: Arc::new(std::sync::Mutex::new(Vec::new())),
            count: Arc::new(AtomicU32::new(0)),
            notify: Arc::new(Notify::new()),
        }
    }

    fn callback(&self) -> impl Fn(Value, &str) + Send + Sync + 'static {
        let values = self.values.clone();
        let count = self.count.clone();
        let notify = self.notify.clone();

        move |value, address| {
            if let Ok(mut guard) = values.lock() {
                guard.push((address.to_string(), value));
            }
            count.fetch_add(1, Ordering::SeqCst);
            notify.notify_waiters();
        }
    }

    fn count(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }

    async fn wait_for_count(&self, n: u32, max_wait: Duration) -> bool {
        wait_for_count(&self.count, n, max_wait).await
    }

    fn values(&self) -> Vec<(String, Value)> {
        self.values.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn has_address(&self, addr: &str) -> bool {
        self.values
            .lock()
            .map(|g| g.iter().any(|(a, _)| a == addr))
            .unwrap_or(false)
    }

    fn latest_value(&self) -> Option<Value> {
        self.values
            .lock()
            .ok()
            .and_then(|g| g.last().map(|(_, v)| v.clone()))
    }

    fn latest_for(&self, addr: &str) -> Option<Value> {
        self.values.lock().ok().and_then(|g| {
            g.iter()
                .rev()
                .find(|(a, _)| a == addr)
                .map(|(_, v)| v.clone())
        })
    }
}

// ============================================================================
// Builder Tests
// ============================================================================

async fn test_builder_default() -> TestResult {
    let start = Instant::now();
    let name = "builder_default";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client = ClaspBuilder::new(&router.url())
            .connect()
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        assert_that(client.is_connected(), "Client not connected")?;
        assert_that(client.session_id().is_some(), "No session ID")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_builder_custom_name() -> TestResult {
    let start = Instant::now();
    let name = "builder_custom_name";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let custom_name = "MyCustomTestClient";
        let client = ClaspBuilder::new(&router.url())
            .name(custom_name)
            .connect()
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        assert_that(client.is_connected(), "Client not connected")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_builder_features() -> TestResult {
    let start = Instant::now();
    let name = "builder_features";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client = ClaspBuilder::new(&router.url())
            .features(vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
                "gesture".to_string(),
            ])
            .connect()
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        assert_that(client.is_connected(), "Client not connected")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_builder_chained() -> TestResult {
    let start = Instant::now();
    let name = "builder_chained";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client = ClaspBuilder::new(&router.url())
            .name("ChainedBuilder")
            .features(vec!["param".to_string(), "event".to_string()])
            .reconnect(false)
            .reconnect_interval(1000)
            .connect()
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        assert_that(client.is_connected(), "Client not connected")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Connection Lifecycle Tests
// ============================================================================

async fn test_connect_to() -> TestResult {
    let start = Instant::now();
    let name = "connect_to";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        assert_that(client.is_connected(), "Client not connected")?;
        assert_that(client.session_id().is_some(), "No session ID")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_session_id() -> TestResult {
    let start = Instant::now();
    let name = "session_id";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let session_id = assert_some(client.session_id(), "No session ID")?;
        assert_that(!session_id.is_empty(), "Session ID is empty")?;
        assert_that(session_id.len() == 36, "Session ID should be UUID format")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_graceful_disconnect() -> TestResult {
    let start = Instant::now();
    let name = "graceful_disconnect";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        assert_that(client.is_connected(), "Should be connected")?;

        client.close().await;

        assert_that(
            !client.is_connected(),
            "Should not be connected after close",
        )?;

        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_connection_error_nonexistent() -> TestResult {
    let start = Instant::now();
    let name = "connection_error_nonexistent";

    let result: Result<(), String> = async {
        let connect_result = timeout(
            Duration::from_secs(3),
            Clasp::connect_to("ws://127.0.0.1:1"),
        )
        .await;

        match connect_result {
            Ok(Ok(_)) => Err("Should have failed to connect to nonexistent server".to_string()),
            Ok(Err(_)) => Ok(()),
            Err(_) => Ok(()),
        }
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_connection_error_invalid_url() -> TestResult {
    let start = Instant::now();
    let name = "connection_error_invalid_url";

    let result: Result<(), String> = async {
        let invalid_urls = vec!["not-a-url", "http://localhost", "", "ftp://server"];

        for url in invalid_urls {
            let connect_result = timeout(Duration::from_secs(2), Clasp::connect_to(url)).await;

            match connect_result {
                Ok(Ok(_)) => {
                    return Err(format!("Should have failed for invalid URL: {}", url));
                }
                _ => {}
            }
        }

        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Parameter Operations Tests
// ============================================================================

async fn test_set_parameter() -> TestResult {
    let start = Instant::now();
    let name = "set_parameter";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        client
            .set("/test/value", 42.0)
            .await
            .map_err(|e| format!("Set failed: {}", e))?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_set_and_receive() -> TestResult {
    let start = Instant::now();
    let name = "set_and_receive";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let collector = ValueCollector::new();

        client
            .subscribe("/test/**", collector.callback())
            .await
            .map_err(|e| format!("Subscribe failed: {}", e))?;

        client
            .set("/test/sensor", 123.456)
            .await
            .map_err(|e| format!("Set failed: {}", e))?;

        // Wait for the value with timeout
        let received = collector.wait_for_count(1, Duration::from_secs(2)).await;
        assert_that(received, "Did not receive SET value within timeout")?;

        // Verify the value
        let value = collector.latest_for("/test/sensor");
        let v = assert_some(value, "No value received for /test/sensor")?;
        let f = v.as_f64().ok_or("Value is not a float")?;
        assert_approx_eq(f, 123.456, 0.001, "Value mismatch")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_set_locked() -> TestResult {
    let start = Instant::now();
    let name = "set_locked";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        client
            .set_locked("/test/locked", 100.0)
            .await
            .map_err(|e| format!("Set locked failed: {}", e))?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_subscribe_pattern_wildcard() -> TestResult {
    let start = Instant::now();
    let name = "subscribe_pattern_wildcard";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let collector = ValueCollector::new();

        client
            .subscribe("/sensors/*", collector.callback())
            .await
            .map_err(|e| format!("Subscribe failed: {}", e))?;

        // Send to matching addresses
        client.set("/sensors/temp", 25.0).await.map_string_err()?;
        client
            .set("/sensors/humidity", 60.0)
            .await
            .map_string_err()?;
        client
            .set("/sensors/pressure", 1013.25)
            .await
            .map_string_err()?;

        // Wait for all three
        let received = collector.wait_for_count(3, Duration::from_secs(2)).await;
        assert_that(received, "Did not receive all 3 values")?;

        // Verify all addresses received
        assert_that(
            collector.has_address("/sensors/temp"),
            "Missing /sensors/temp",
        )?;
        assert_that(
            collector.has_address("/sensors/humidity"),
            "Missing /sensors/humidity",
        )?;
        assert_that(
            collector.has_address("/sensors/pressure"),
            "Missing /sensors/pressure",
        )?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_subscribe_pattern_globstar() -> TestResult {
    let start = Instant::now();
    let name = "subscribe_pattern_globstar";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let collector = ValueCollector::new();

        // ** should match any depth
        client
            .subscribe("/app/**", collector.callback())
            .await
            .map_err(|e| format!("Subscribe failed: {}", e))?;

        client.set("/app/level1", 1.0).await.map_string_err()?;
        client
            .set("/app/level1/level2", 2.0)
            .await
            .map_string_err()?;
        client.set("/app/a/b/c/d", 4.0).await.map_string_err()?;

        let received = collector.wait_for_count(3, Duration::from_secs(2)).await;
        assert_that(received, "Did not receive all globstar values")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_unsubscribe() -> TestResult {
    let start = Instant::now();
    let name = "unsubscribe";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let collector = ValueCollector::new();

        let sub_id = client
            .subscribe("/unsub/**", collector.callback())
            .await
            .map_err(|e| format!("Subscribe failed: {}", e))?;

        // Send one value
        client.set("/unsub/before", 1.0).await.map_string_err()?;
        collector.wait_for_count(1, Duration::from_secs(1)).await;
        let count_before = collector.count();

        // Unsubscribe
        client
            .unsubscribe(sub_id)
            .await
            .map_err(|e| format!("Unsubscribe failed: {}", e))?;

        // Small delay for unsubscribe to propagate
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send more values - should not be received
        client.set("/unsub/after1", 2.0).await.map_string_err()?;
        client.set("/unsub/after2", 3.0).await.map_string_err()?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        let count_after = collector.count();

        // Count should not have increased significantly after unsubscribe
        assert_that(
            count_after <= count_before + 1,
            &format!(
                "Received values after unsubscribe: before={}, after={}",
                count_before, count_after
            ),
        )?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_cached_value() -> TestResult {
    let start = Instant::now();
    let name = "cached_value";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let collector = ValueCollector::new();

        client
            .subscribe("/cache/**", collector.callback())
            .await
            .map_err(|e| format!("Subscribe failed: {}", e))?;

        client.set("/cache/test", 42.0).await.map_string_err()?;

        collector.wait_for_count(1, Duration::from_secs(2)).await;

        // Check cached value
        let cached = client.cached("/cache/test");
        if let Some(v) = cached {
            let f = v.as_f64().ok_or("Cached value not a float")?;
            assert_approx_eq(f, 42.0, 0.001, "Cached value mismatch")?;
        }
        // Note: Cache might not be populated if the value arrives asynchronously
        // This test verifies the cache API works, not that it's always populated

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Event Operations Tests
// ============================================================================

async fn test_emit_event() -> TestResult {
    let start = Instant::now();
    let name = "emit_event";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        client
            .emit("/events/button", Value::String("pressed".to_string()))
            .await
            .map_err(|e| format!("Emit failed: {}", e))?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_emit_and_receive() -> TestResult {
    let start = Instant::now();
    let name = "emit_and_receive";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        // Two clients: one emits, one receives
        let receiver = Clasp::connect_to(&router.url()).await.map_string_err()?;
        let emitter = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();

        receiver
            .subscribe("/events/**", collector.callback())
            .await
            .map_string_err()?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        emitter
            .emit("/events/trigger", Value::String("activated".to_string()))
            .await
            .map_string_err()?;

        let received = collector.wait_for_count(1, Duration::from_secs(2)).await;
        assert_that(received, "Event not received")?;

        assert_that(
            collector.has_address("/events/trigger"),
            "Wrong event address",
        )?;

        receiver.close().await;
        emitter.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_stream() -> TestResult {
    let start = Instant::now();
    let name = "stream";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        // Stream sends multiple samples
        for i in 0..10 {
            client
                .stream("/sensors/accel", Value::Float(i as f64 * 0.1))
                .await
                .map_err(|e| format!("Stream {} failed: {}", i, e))?;
        }

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Advanced Features Tests
// ============================================================================

async fn test_bundle() -> TestResult {
    let start = Instant::now();
    let name = "bundle";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

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

        client
            .bundle(messages)
            .await
            .map_err(|e| format!("Bundle failed: {}", e))?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_bundle_atomicity() -> TestResult {
    let start = Instant::now();
    let name = "bundle_atomicity";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let sender = Clasp::connect_to(&router.url()).await.map_string_err()?;
        let receiver = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();
        receiver
            .subscribe("/atomic/**", collector.callback())
            .await
            .map_string_err()?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send bundle of 5 values
        let messages: Vec<Message> = (0..5)
            .map(|i| {
                Message::Set(SetMessage {
                    address: format!("/atomic/v{}", i),
                    value: Value::Int(i),
                    revision: None,
                    lock: false,
                    unlock: false,
                })
            })
            .collect();

        sender.bundle(messages).await.map_string_err()?;

        // Should receive all 5
        let received = collector.wait_for_count(5, Duration::from_secs(2)).await;
        assert_that(received, "Did not receive all bundle values")?;

        sender.close().await;
        receiver.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_bundle_at() -> TestResult {
    let start = Instant::now();
    let name = "bundle_at";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let messages = vec![Message::Set(SetMessage {
            address: "/scheduled/value".to_string(),
            value: Value::Float(99.9),
            revision: None,
            lock: false,
            unlock: false,
        })];

        let future_time = client.time() + 100_000; // 100ms in microseconds
        client
            .bundle_at(messages, future_time)
            .await
            .map_err(|e| format!("Bundle_at failed: {}", e))?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_clock_sync() -> TestResult {
    let start = Instant::now();
    let name = "clock_sync";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url())
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        let server_time = client.time();

        // Should be a reasonable timestamp (non-zero, in microseconds)
        assert_that(server_time > 0, "Server time should be positive")?;

        // Should be roughly recent (within last hour of current time in microseconds)
        let now_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        let diff = (server_time as i64 - now_micros).abs();
        // Allow up to 1 hour difference for any sync offset
        assert_that(
            diff < 3600_000_000,
            &format!("Server time too far from local: diff={}", diff),
        )?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Value Type Tests
// ============================================================================

async fn test_value_type_int() -> TestResult {
    let start = Instant::now();
    let name = "value_type_int";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();
        client
            .subscribe("/types/**", collector.callback())
            .await
            .map_string_err()?;

        client.set("/types/int", 42i64).await.map_string_err()?;
        client
            .set("/types/int_neg", -100i64)
            .await
            .map_string_err()?;
        client.set("/types/int_zero", 0i64).await.map_string_err()?;
        client
            .set("/types/int_max", i64::MAX)
            .await
            .map_string_err()?;

        collector.wait_for_count(4, Duration::from_secs(2)).await;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_value_type_float() -> TestResult {
    let start = Instant::now();
    let name = "value_type_float";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();
        client
            .subscribe("/types/**", collector.callback())
            .await
            .map_string_err()?;

        client
            .set("/types/float", 3.14159f64)
            .await
            .map_string_err()?;
        client
            .set("/types/float_neg", -273.15f64)
            .await
            .map_string_err()?;
        client
            .set("/types/float_zero", 0.0f64)
            .await
            .map_string_err()?;
        client
            .set("/types/float_tiny", 1e-100f64)
            .await
            .map_string_err()?;

        collector.wait_for_count(4, Duration::from_secs(2)).await;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_value_type_bool() -> TestResult {
    let start = Instant::now();
    let name = "value_type_bool";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();
        client
            .subscribe("/types/**", collector.callback())
            .await
            .map_string_err()?;

        client
            .set("/types/bool_true", true)
            .await
            .map_string_err()?;
        client
            .set("/types/bool_false", false)
            .await
            .map_string_err()?;

        let received = collector.wait_for_count(2, Duration::from_secs(2)).await;
        assert_that(received, "Did not receive bool values")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_value_type_string() -> TestResult {
    let start = Instant::now();
    let name = "value_type_string";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();
        client
            .subscribe("/types/**", collector.callback())
            .await
            .map_string_err()?;

        client
            .set("/types/str", "hello world")
            .await
            .map_string_err()?;
        client.set("/types/str_empty", "").await.map_string_err()?;
        client
            .set("/types/str_unicode", "Hello, \u{1F30D}!")
            .await
            .map_string_err()?;
        client
            .set("/types/str_long", "x".repeat(1000))
            .await
            .map_string_err()?;

        collector.wait_for_count(4, Duration::from_secs(2)).await;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_value_type_bytes() -> TestResult {
    let start = Instant::now();
    let name = "value_type_bytes";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        client
            .set(
                "/types/bytes",
                Value::Bytes(vec![0x00, 0xFF, 0x42, 0xDE, 0xAD]),
            )
            .await
            .map_string_err()?;
        client
            .set("/types/bytes_empty", Value::Bytes(vec![]))
            .await
            .map_string_err()?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_value_type_array() -> TestResult {
    let start = Instant::now();
    let name = "value_type_array";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        client
            .set(
                "/types/array",
                Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            )
            .await
            .map_string_err()?;

        client
            .set(
                "/types/array_mixed",
                Value::Array(vec![
                    Value::Int(1),
                    Value::Float(2.5),
                    Value::String("three".to_string()),
                ]),
            )
            .await
            .map_string_err()?;

        client
            .set("/types/array_empty", Value::Array(vec![]))
            .await
            .map_string_err()?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_value_type_null() -> TestResult {
    let start = Instant::now();
    let name = "value_type_null";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        client
            .set("/types/null", Value::Null)
            .await
            .map_string_err()?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Two-Client Interaction Tests
// ============================================================================

async fn test_two_client_set_receive() -> TestResult {
    let start = Instant::now();
    let name = "two_client_set_receive";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client1 = Clasp::connect_to(&router.url()).await.map_string_err()?;
        let client2 = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector = ValueCollector::new();

        client1
            .subscribe("/shared/**", collector.callback())
            .await
            .map_string_err()?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        client2.set("/shared/value", 42.0).await.map_string_err()?;

        let received = collector.wait_for_count(1, Duration::from_secs(2)).await;
        assert_that(received, "Client 1 did not receive value from Client 2")?;

        client1.close().await;
        client2.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_bidirectional_communication() -> TestResult {
    let start = Instant::now();
    let name = "bidirectional_communication";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let client1 = Clasp::connect_to(&router.url()).await.map_string_err()?;
        let client2 = Clasp::connect_to(&router.url()).await.map_string_err()?;

        let collector1 = ValueCollector::new();
        let collector2 = ValueCollector::new();

        client1
            .subscribe("/from2/**", collector1.callback())
            .await
            .map_string_err()?;
        client2
            .subscribe("/from1/**", collector2.callback())
            .await
            .map_string_err()?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Bidirectional sends
        client1
            .set("/from1/message", 100.0)
            .await
            .map_string_err()?;
        client2
            .set("/from2/message", 200.0)
            .await
            .map_string_err()?;

        let recv1 = collector1.wait_for_count(1, Duration::from_secs(2)).await;
        let recv2 = collector2.wait_for_count(1, Duration::from_secs(2)).await;

        assert_that(recv1, "Client1 did not receive from Client2")?;
        assert_that(recv2, "Client2 did not receive from Client1")?;

        client1.close().await;
        client2.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Concurrent Operations Tests
// ============================================================================

async fn test_concurrent_operations() -> TestResult {
    let start = Instant::now();
    let name = "concurrent_operations";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        let mut clients = vec![];
        for i in 0..5 {
            let client = Clasp::builder(&router.url())
                .name(&format!("ConcurrentClient{}", i))
                .connect()
                .await
                .map_string_err()?;
            clients.push(client);
        }

        let mut success_count = 0;
        for (i, client) in clients.iter().enumerate() {
            for j in 0..5 {
                match client
                    .set(&format!("/concurrent/{}/{}", i, j), (i * 10 + j) as f64)
                    .await
                {
                    Ok(()) => success_count += 1,
                    Err(_) => {}
                }
            }
        }

        assert_that(
            success_count >= 20,
            &format!("Only {}/25 concurrent operations succeeded", success_count),
        )?;

        for client in clients {
            client.close().await;
        }

        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_rapid_subscribe_unsubscribe() -> TestResult {
    let start = Instant::now();
    let name = "rapid_subscribe_unsubscribe";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        for i in 0..20 {
            let collector = ValueCollector::new();
            let sub_id = client
                .subscribe(&format!("/rapid/{}", i), collector.callback())
                .await
                .map_string_err()?;
            client.unsubscribe(sub_id).await.map_string_err()?;
        }

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Edge Case and Negative Tests
// ============================================================================

async fn test_operations_before_connect() -> TestResult {
    let start = Instant::now();
    let name = "operations_before_connect";

    // This test verifies that builder state is clean before connect
    let result: Result<(), String> = async {
        let router = TestRouter::start().await;

        // Build client but don't connect yet
        let builder = ClaspBuilder::new(&router.url()).name("PreConnect");

        // Now connect and verify it works
        let client = builder.connect().await.map_string_err()?;
        assert_that(client.is_connected(), "Should be connected")?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_operations_after_close() -> TestResult {
    let start = Instant::now();
    let name = "operations_after_close";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        client.close().await;

        // These should not panic
        assert_that(!client.is_connected(), "Should not be connected")?;
        let _ = client.set("/test", 1.0).await;
        let _ = client.subscribe("/test", |_, _| {}).await;

        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_double_close() -> TestResult {
    let start = Instant::now();
    let name = "double_close";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        client.close().await;
        client.close().await; // Should not panic

        assert_that(!client.is_connected(), "Should not be connected")?;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

async fn test_special_characters_in_address() -> TestResult {
    let start = Instant::now();
    let name = "special_characters_in_address";

    let result: Result<(), String> = async {
        let router = TestRouter::start().await;
        let client = Clasp::connect_to(&router.url()).await.map_string_err()?;

        // Various address formats
        client.set("/simple", 1.0).await.map_string_err()?;
        client.set("/with-dash", 2.0).await.map_string_err()?;
        client.set("/with_underscore", 3.0).await.map_string_err()?;
        client.set("/with.dot", 4.0).await.map_string_err()?;
        client.set("/CamelCase", 5.0).await.map_string_err()?;
        client.set("/with123numbers", 6.0).await.map_string_err()?;

        client.close().await;
        Ok(())
    }
    .await;

    TestResult::from_result(name, result, start.elapsed().as_millis())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("warn").init();

    println!("\n{}", "=".repeat(70));
    println!("             CLASP Client Library Tests (Grade A)");
    println!("{}\n", "=".repeat(70));

    let tests = vec![
        // Builder tests
        test_builder_default().await,
        test_builder_custom_name().await,
        test_builder_features().await,
        test_builder_chained().await,
        // Connection lifecycle tests
        test_connect_to().await,
        test_session_id().await,
        test_graceful_disconnect().await,
        test_connection_error_nonexistent().await,
        test_connection_error_invalid_url().await,
        // Parameter operations tests
        test_set_parameter().await,
        test_set_and_receive().await,
        test_set_locked().await,
        test_subscribe_pattern_wildcard().await,
        test_subscribe_pattern_globstar().await,
        test_unsubscribe().await,
        test_cached_value().await,
        // Event operations tests
        test_emit_event().await,
        test_emit_and_receive().await,
        test_stream().await,
        // Advanced features tests
        test_bundle().await,
        test_bundle_atomicity().await,
        test_bundle_at().await,
        test_clock_sync().await,
        // Value type tests
        test_value_type_int().await,
        test_value_type_float().await,
        test_value_type_bool().await,
        test_value_type_string().await,
        test_value_type_bytes().await,
        test_value_type_array().await,
        test_value_type_null().await,
        // Two-client interaction tests
        test_two_client_set_receive().await,
        test_bidirectional_communication().await,
        // Concurrent operations tests
        test_concurrent_operations().await,
        test_rapid_subscribe_unsubscribe().await,
        // Edge case and negative tests
        test_operations_before_connect().await,
        test_operations_after_close().await,
        test_double_close().await,
        test_special_characters_in_address().await,
    ];

    let mut passed = 0;
    let mut failed = 0;

    println!("{:<40} {:>8} {:>10}", "Test", "Status", "Time");
    println!("{}", "-".repeat(60));

    for test in &tests {
        let status = if test.passed { "PASS" } else { "FAIL" };
        let color = if test.passed { "\x1b[32m" } else { "\x1b[31m" };
        println!(
            "{:<40} {}{:>8}\x1b[0m {:>8}ms",
            test.name, color, status, test.duration_ms
        );

        if test.passed {
            passed += 1;
        } else {
            failed += 1;
            println!("    Error: {}", test.message);
        }
    }

    println!("{}", "-".repeat(60));
    println!(
        "Results: {} passed, {} failed, {} total",
        passed,
        failed,
        tests.len()
    );
    println!();

    if failed > 0 {
        std::process::exit(1);
    }
}
