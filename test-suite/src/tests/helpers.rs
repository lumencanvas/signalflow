//! Common test helpers and utilities

use crate::TestResult;
use std::time::{Duration, Instant};
use tokio::time::timeout;

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

/// Wait for a condition with timeout
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
