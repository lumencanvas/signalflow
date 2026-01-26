//! Common test helpers and utilities for CLASP tests
//!
//! This crate provides robust test utilities including:
//! - Condition-based waiting (no hardcoded sleeps)
//! - Proper resource cleanup with RAII
//! - Strong assertion helpers
//! - Test router management
//! - Value collectors for subscription testing

use clasp_client::Clasp;
use clasp_core::{SecurityMode, Value};
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::timeout;

/// Default test timeout
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default condition check interval
pub const DEFAULT_CHECK_INTERVAL: Duration = Duration::from_millis(10);

// ============================================================================
// Port Allocation
// ============================================================================

/// Find an available TCP port for testing
pub async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

/// Find an available UDP port for testing
pub fn find_available_udp_port() -> u16 {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

// ============================================================================
// Condition-Based Waiting
// ============================================================================

/// Wait for a condition with timeout - condition-based, not time-based
pub async fn wait_for<F, Fut>(check: F, interval: Duration, max_wait: Duration) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    while start.elapsed() < max_wait {
        if check().await {
            return true;
        }
        tokio::time::sleep(interval).await;
    }
    false
}

/// Wait for an atomic counter to reach a target value
pub async fn wait_for_count(counter: &AtomicU32, target: u32, max_wait: Duration) -> bool {
    wait_for(
        || async { counter.load(Ordering::SeqCst) >= target },
        DEFAULT_CHECK_INTERVAL,
        max_wait,
    )
    .await
}

/// Wait for a boolean flag to become true
pub async fn wait_for_flag(flag: &AtomicBool, max_wait: Duration) -> bool {
    wait_for(
        || async { flag.load(Ordering::SeqCst) },
        DEFAULT_CHECK_INTERVAL,
        max_wait,
    )
    .await
}

/// Wait with notification - more efficient than polling
pub async fn wait_with_notify(notify: &Notify, max_wait: Duration) -> bool {
    timeout(max_wait, notify.notified()).await.is_ok()
}

// ============================================================================
// Test Router - RAII wrapper with proper cleanup
// ============================================================================

/// A test router that automatically cleans up on drop
pub struct TestRouter {
    port: u16,
    handle: Option<tokio::task::JoinHandle<()>>,
    ready: Arc<AtomicBool>,
}

impl TestRouter {
    /// Start a test router with default configuration
    pub async fn start() -> Self {
        Self::start_with_config(RouterConfig {
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
            gesture_coalescing: true,
            gesture_coalesce_interval_ms: 16,
            max_messages_per_second: 0, // Disable rate limiting for tests
            rate_limiting_enabled: false,
            state_config: clasp_router::RouterStateConfig::unlimited(), // No TTL in tests
        })
        .await
    }

    /// Start a test router with custom configuration
    pub async fn start_with_config(config: RouterConfig) -> Self {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);
        let ready = Arc::new(AtomicBool::new(false));
        let ready_clone = ready.clone();

        let router = Router::new(config);

        let handle = tokio::spawn(async move {
            ready_clone.store(true, Ordering::SeqCst);
            let _ = router.serve_websocket(&addr).await;
        });

        // Wait for router to be ready using condition-based wait
        let start = Instant::now();
        while !ready.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(5) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Additional check: try to connect to verify the port is listening
        let _ = wait_for(
            || {
                let port = port;
                async move {
                    tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                        .await
                        .is_ok()
                }
            },
            Duration::from_millis(10),
            Duration::from_secs(5),
        )
        .await;

        Self {
            port,
            handle: Some(handle),
            ready,
        }
    }

    /// Get the WebSocket URL for this router
    pub fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    /// Get the port number
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Check if router is ready
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    /// Connect a client to this router
    pub async fn connect_client(&self) -> Result<Clasp, clasp_client::ClientError> {
        Clasp::connect_to(&self.url()).await
    }

    /// Connect a client with a custom name
    pub async fn connect_client_named(
        &self,
        name: &str,
    ) -> Result<Clasp, clasp_client::ClientError> {
        Clasp::builder(&self.url()).name(name).connect().await
    }

    /// Stop the router explicitly (also happens on drop)
    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

impl Drop for TestRouter {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that two values are approximately equal (for floating point)
pub fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64, msg: &str) -> Result<(), String> {
    if (actual - expected).abs() < epsilon {
        Ok(())
    } else {
        Err(format!(
            "{}: expected {} +/- {}, got {}",
            msg, expected, epsilon, actual
        ))
    }
}

/// Assert a condition with a custom message
pub fn assert_that(condition: bool, msg: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

/// Assert that an Option is Some and return the value
pub fn assert_some<T>(opt: Option<T>, msg: &str) -> Result<T, String> {
    opt.ok_or_else(|| msg.to_string())
}

/// Assert that a Result is Ok and return the value
pub fn assert_ok<T, E: std::fmt::Debug>(result: Result<T, E>, msg: &str) -> Result<T, String> {
    result.map_err(|e| format!("{}: {:?}", msg, e))
}

/// Assert that a Result is Err
pub fn assert_err<T: std::fmt::Debug, E>(result: Result<T, E>, msg: &str) -> Result<(), String> {
    match result {
        Ok(v) => Err(format!("{}: expected error, got Ok({:?})", msg, v)),
        Err(_) => Ok(()),
    }
}

// ============================================================================
// Test Collectors - for verifying received values
// ============================================================================

/// Collector for subscription values with thread-safe access
#[derive(Clone)]
pub struct ValueCollector {
    values: Arc<parking_lot::Mutex<Vec<(String, Value)>>>,
    notify: Arc<Notify>,
    count: Arc<AtomicU32>,
}

impl ValueCollector {
    pub fn new() -> Self {
        Self {
            values: Arc::new(parking_lot::Mutex::new(Vec::new())),
            notify: Arc::new(Notify::new()),
            count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Create a callback function for subscriptions
    pub fn callback(&self) -> impl Fn(Value, String) + Send + 'static {
        let values = self.values.clone();
        let notify = self.notify.clone();
        let count = self.count.clone();

        move |value, address| {
            {
                let mut guard = values.lock();
                guard.push((address, value));
            }
            count.fetch_add(1, Ordering::SeqCst);
            notify.notify_waiters();
        }
    }

    /// Create a callback function for subscriptions that takes &str address
    pub fn callback_ref(&self) -> impl Fn(Value, &str) + Send + Sync + 'static {
        let values = self.values.clone();
        let notify = self.notify.clone();
        let count = self.count.clone();

        move |value, address| {
            {
                let mut guard = values.lock();
                guard.push((address.to_string(), value));
            }
            count.fetch_add(1, Ordering::SeqCst);
            notify.notify_waiters();
        }
    }

    /// Get the count of received values
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }

    /// Wait for at least n values to be received
    pub async fn wait_for_count(&self, n: u32, max_wait: Duration) -> bool {
        wait_for_count(&self.count, n, max_wait).await
    }

    /// Get all collected values
    pub fn values(&self) -> Vec<(String, Value)> {
        self.values.lock().clone()
    }

    /// Check if a specific address was received
    pub fn has_address(&self, addr: &str) -> bool {
        self.values.lock().iter().any(|(a, _)| a == addr)
    }

    /// Get values for a specific address pattern
    pub fn values_for(&self, addr: &str) -> Vec<Value> {
        self.values
            .lock()
            .iter()
            .filter(|(a, _)| a == addr)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Get the last value received
    pub fn last_value(&self) -> Option<(String, Value)> {
        self.values.lock().last().cloned()
    }

    /// Clear all collected values
    pub fn clear(&self) {
        self.values.lock().clear();
        self.count.store(0, Ordering::SeqCst);
    }
}

impl Default for ValueCollector {
    fn default() -> Self {
        Self::new()
    }
}
