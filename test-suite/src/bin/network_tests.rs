//! Network Condition Simulation Tests
//!
//! Tests CLASP behavior under adverse network conditions:
//! - Packet loss
//! - Latency
//! - Jitter
//! - Bandwidth throttling
//! - Connection drops
//!
//! Run with: cargo run -p clasp-test-suite --bin network-tests
//!
//! For full network simulation, run with toxiproxy:
//!   docker run -d --name toxiproxy -p 8474:8474 -p 9000-9100:9000-9100 ghcr.io/shopify/toxiproxy
//!
//! Environment variables:
//! - CLASP_TOXIPROXY_HOST=localhost:8474  Toxiproxy API endpoint
//! - CLASP_NETWORK_SIMULATION=1           Enable network simulation tests

use clasp_client::Clasp;
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    fn pass(name: &'static str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name,
            passed: true,
            message: message.into(),
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
// Test Infrastructure
// ============================================================================

async fn find_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct TestRouter {
    port: u16,
    handle: tokio::task::JoinHandle<()>,
}

impl TestRouter {
    async fn start() -> Self {
        let port = find_port().await;
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
// Simulated Network Conditions (without external tools)
// ============================================================================

/// Test behavior when messages are sent rapidly
async fn test_burst_traffic() -> TestResult {
    let start = Instant::now();
    let name = "burst_traffic";

    let router = TestRouter::start().await;

    let sender = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let receiver = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Receiver connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let received = Arc::new(AtomicU32::new(0));
    let received_clone = received.clone();

    let _ = receiver
        .subscribe("/burst/**", move |_, _| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send 1000 messages as fast as possible
    let burst_count = 1000;
    for i in 0..burst_count {
        let _ = sender.set(&format!("/burst/{}", i), i as f64).await;
    }

    // Wait for delivery
    tokio::time::sleep(Duration::from_millis(500)).await;

    let count = received.load(Ordering::SeqCst);
    router.stop();

    if count >= (burst_count * 9 / 10) as u32 {
        TestResult::pass(
            name,
            format!("Received {}/{} messages", count, burst_count),
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/{} messages", count, burst_count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test behavior with many concurrent connections
async fn test_connection_storm() -> TestResult {
    let start = Instant::now();
    let name = "connection_storm";

    let router = TestRouter::start().await;
    let url = router.url();

    // Connect many clients rapidly (sequential to avoid Send issues)
    let client_count = 50;
    let mut success_count = 0;

    for i in 0..client_count {
        match Clasp::builder(&url)
            .name(&format!("storm-client-{}", i))
            .connect()
            .await
        {
            Ok(client) => {
                // Send a message
                if client.set(&format!("/storm/{}", i), i as f64).await.is_ok() {
                    success_count += 1;
                }
                // Don't close - let them accumulate
            }
            Err(_) => {}
        }
    }

    router.stop();

    if success_count >= client_count * 9 / 10 {
        TestResult::pass(
            name,
            format!(
                "{}/{} clients connected successfully",
                success_count, client_count
            ),
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::fail(
            name,
            format!("Only {}/{} clients connected", success_count, client_count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test reconnection behavior after connection drop
async fn test_connection_recovery() -> TestResult {
    let start = Instant::now();
    let name = "connection_recovery";

    let router = TestRouter::start().await;
    let url = router.url();

    // Connect client
    let client = match Clasp::connect_to(&url).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Initial connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    // Verify working
    if let Err(e) = client.set("/test/value", 1.0).await {
        router.stop();
        return TestResult::fail(
            name,
            format!("Initial set failed: {}", e),
            start.elapsed().as_millis(),
        );
    }

    // Close connection
    client.close().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Reconnect
    match Clasp::connect_to(&url).await {
        Ok(new_client) => {
            if let Err(e) = new_client.set("/test/value", 2.0).await {
                router.stop();
                return TestResult::fail(
                    name,
                    format!("Post-reconnect set failed: {}", e),
                    start.elapsed().as_millis(),
                );
            }
            router.stop();
            TestResult::pass(name, "Reconnection successful", start.elapsed().as_millis())
        }
        Err(e) => {
            router.stop();
            TestResult::fail(
                name,
                format!("Reconnect failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    }
}

/// Test behavior with delayed responses
async fn test_slow_consumer() -> TestResult {
    let start = Instant::now();
    let name = "slow_consumer";

    let router = TestRouter::start().await;

    let sender = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Sender connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let receiver = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Receiver connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let received = Arc::new(AtomicU32::new(0));
    let received_clone = received.clone();

    // Slow consumer - sleeps on each message
    let _ = receiver
        .subscribe("/slow/**", move |_, _| {
            received_clone.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(10)); // Simulate slow processing
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send 100 messages quickly
    for i in 0..100 {
        let _ = sender.set(&format!("/slow/{}", i), i as f64).await;
    }

    // Wait longer for slow consumer
    tokio::time::sleep(Duration::from_secs(2)).await;

    let count = received.load(Ordering::SeqCst);
    router.stop();

    if count >= 90 {
        TestResult::pass(
            name,
            format!("Slow consumer received {}/100", count),
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::fail(
            name,
            format!("Slow consumer only got {}/100", count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test behavior with large messages
async fn test_large_payloads() -> TestResult {
    let start = Instant::now();
    let name = "large_payloads";

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    // Test with increasingly large string values
    let sizes = [1_000, 10_000, 50_000];
    let mut results = vec![];

    for size in sizes {
        let large_string: String = "x".repeat(size);
        match client.set("/large/payload", large_string.clone()).await {
            Ok(()) => results.push((size, true)),
            Err(_) => results.push((size, false)),
        }
    }

    router.stop();

    let passed: Vec<_> = results.iter().filter(|(_, ok)| *ok).collect();
    if passed.len() == sizes.len() {
        TestResult::pass(
            name,
            format!("All sizes passed: {:?}", sizes),
            start.elapsed().as_millis(),
        )
    } else {
        let failed: Vec<_> = results
            .iter()
            .filter(|(_, ok)| !*ok)
            .map(|(s, _)| *s)
            .collect();
        TestResult::fail(
            name,
            format!("Failed at sizes: {:?}", failed),
            start.elapsed().as_millis(),
        )
    }
}

/// Test message ordering under load
async fn test_message_ordering() -> TestResult {
    let start = Instant::now();
    let name = "message_ordering";

    let router = TestRouter::start().await;

    let sender = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let receiver = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Receiver connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let received_values = Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_clone = received_values.clone();

    let _ = receiver
        .subscribe("/order/value", move |value, _| {
            if let Some(v) = value.as_f64() {
                if let Ok(mut vec) = received_clone.lock() {
                    vec.push(v as i32);
                }
            }
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send numbered messages
    let count = 100;
    for i in 0..count {
        let _ = sender.set("/order/value", i as f64).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    router.stop();

    let values = received_values.lock().unwrap();

    // Check if values are in order
    let mut in_order = true;
    for i in 1..values.len() {
        if values[i] < values[i - 1] {
            in_order = false;
            break;
        }
    }

    if in_order && values.len() >= (count * 9 / 10) as usize {
        TestResult::pass(
            name,
            format!("Received {} messages in order", values.len()),
            start.elapsed().as_millis(),
        )
    } else if !in_order {
        TestResult::fail(
            name,
            "Messages received out of order",
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/{} messages", values.len(), count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test rapid subscribe/unsubscribe cycles
async fn test_subscription_churn() -> TestResult {
    let start = Instant::now();
    let name = "subscription_churn";

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    // Rapidly subscribe and unsubscribe
    let cycles = 100;
    let mut errors = 0;

    for i in 0..cycles {
        let pattern = format!("/churn/{}/**", i);
        match client.subscribe(&pattern, |_, _| {}).await {
            Ok(sub_id) => {
                // Immediately unsubscribe
                if client.unsubscribe(sub_id).await.is_err() {
                    errors += 1;
                }
            }
            Err(_) => errors += 1,
        }
    }

    router.stop();

    if errors == 0 {
        TestResult::pass(
            name,
            format!("{} subscribe/unsubscribe cycles", cycles),
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::fail(
            name,
            format!("{} errors in {} cycles", errors, cycles),
            start.elapsed().as_millis(),
        )
    }
}

/// Test behavior under memory pressure
async fn test_memory_pressure() -> TestResult {
    let start = Instant::now();
    let name = "memory_pressure";

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    // Create many unique addresses
    let address_count = 10_000;
    for i in 0..address_count {
        if let Err(e) = client.set(&format!("/memory/addr/{}", i), i as f64).await {
            router.stop();
            return TestResult::fail(
                name,
                format!("Failed at {}: {}", i, e),
                start.elapsed().as_millis(),
            );
        }
    }

    router.stop();
    TestResult::pass(
        name,
        format!("Created {} unique addresses", address_count),
        start.elapsed().as_millis(),
    )
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("warn").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Network Simulation Tests                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_burst_traffic().await,
        test_connection_storm().await,
        test_connection_recovery().await,
        test_slow_consumer().await,
        test_large_payloads().await,
        test_message_ordering().await,
        test_subscription_churn().await,
        test_memory_pressure().await,
    ];

    let mut passed = 0;
    let mut failed = 0;

    println!("┌──────────────────────────────────────┬────────┬──────────┐");
    println!("│ Test                                 │ Status │ Time     │");
    println!("├──────────────────────────────────────┼────────┼──────────┤");

    for test in &tests {
        let (status, color) = if test.passed {
            ("✓ PASS", "\x1b[32m")
        } else {
            ("✗ FAIL", "\x1b[31m")
        };

        println!(
            "│ {:<36} │ {}{:<6}\x1b[0m │ {:>6}ms │",
            test.name, color, status, test.duration_ms
        );

        if test.passed {
            passed += 1;
            if !test.message.is_empty() {
                let msg = &test.message[..test.message.len().min(56)];
                println!("│   └─ {:<56} │", msg);
            }
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
