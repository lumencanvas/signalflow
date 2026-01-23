//! Common test helpers and utilities
//!
//! This module provides robust, Grade-A quality test utilities including:
//! - Condition-based waiting (no hardcoded sleeps)
//! - Proper resource cleanup with RAII
//! - Strong assertion helpers
//! - Test router management

use crate::TestResult;
use clasp_client::Clasp;
use clasp_core::SecurityMode;
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

/// Run a test with timeout and capture results
pub async fn run_test<F, Fut>(name: &str, timeout_duration: Duration, test_fn: F) -> TestResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let start = Instant::now();

    let result = match timeout(timeout_duration, test_fn()).await {
        Ok(Ok(())) => TestResult {
            name: name.to_string(),
            passed: true,
            duration: start.elapsed(),
            message: None,
        },
        Ok(Err(e)) => TestResult {
            name: name.to_string(),
            passed: false,
            duration: start.elapsed(),
            message: Some(e),
        },
        Err(_) => TestResult {
            name: name.to_string(),
            passed: false,
            duration: start.elapsed(),
            message: Some(format!("Test timed out after {:?}", timeout_duration)),
        },
    };

    result
}

/// Find an available port for testing
pub async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

/// Find an available UDP port for testing
pub fn find_available_udp_port() -> u16 {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

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
    values: Arc<std::sync::Mutex<Vec<(String, clasp_core::Value)>>>,
    notify: Arc<Notify>,
    count: Arc<AtomicU32>,
}

impl ValueCollector {
    pub fn new() -> Self {
        Self {
            values: Arc::new(std::sync::Mutex::new(Vec::new())),
            notify: Arc::new(Notify::new()),
            count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Create a callback function for subscriptions
    pub fn callback(&self) -> impl Fn(clasp_core::Value, String) + Send + 'static {
        let values = self.values.clone();
        let notify = self.notify.clone();
        let count = self.count.clone();

        move |value, address| {
            if let Ok(mut guard) = values.lock() {
                guard.push((address, value));
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
    pub fn values(&self) -> Vec<(String, clasp_core::Value)> {
        self.values.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Check if a specific address was received
    pub fn has_address(&self, addr: &str) -> bool {
        self.values
            .lock()
            .map(|g| g.iter().any(|(a, _)| a == addr))
            .unwrap_or(false)
    }

    /// Get values for a specific address pattern
    pub fn values_for(&self, addr: &str) -> Vec<clasp_core::Value> {
        self.values
            .lock()
            .map(|g| {
                g.iter()
                    .filter(|(a, _)| a == addr)
                    .map(|(_, v)| v.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear all collected values
    pub fn clear(&self) {
        if let Ok(mut guard) = self.values.lock() {
            guard.clear();
        }
        self.count.store(0, Ordering::SeqCst);
    }
}

impl Default for ValueCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Test Result Builder
// ============================================================================

pub struct TestResultBuilder {
    name: &'static str,
    start: Instant,
}

impl TestResultBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }

    pub fn pass(self) -> crate::TestResult {
        crate::TestResult {
            name: self.name.to_string(),
            passed: true,
            duration: self.start.elapsed(),
            message: None,
        }
    }

    pub fn fail(self, msg: impl Into<String>) -> crate::TestResult {
        crate::TestResult {
            name: self.name.to_string(),
            passed: false,
            duration: self.start.elapsed(),
            message: Some(msg.into()),
        }
    }

    pub fn from_result(self, result: Result<(), String>) -> crate::TestResult {
        match result {
            Ok(()) => self.pass(),
            Err(msg) => self.fail(msg),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}
