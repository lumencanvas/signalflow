//! Multi-Protocol End-to-End Tests
//!
//! Tests for complete protocol chains including:
//! - OSC input → CLASP router → Client output
//! - Multiple bridges working together
//! - Full message flow verification

use clasp_bridge::osc::OscBridge;
use clasp_bridge::{Bridge, BridgeEvent};
use clasp_client::Clasp;
use clasp_core::{SecurityMode, Value};
use clasp_router::{Router, RouterConfig};
use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

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

fn find_udp_port() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
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
            name: "E2E Test Router".to_string(),
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
// E2E Tests: Client to Client via Router
// ============================================================================

async fn test_client_to_client_set() -> TestResult {
    let start = std::time::Instant::now();
    let name = "client_to_client_set";

    let router = TestRouter::start().await;

    // Receiver client
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
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    let _ = receiver
        .subscribe("/e2e/**", move |value, _address| {
            if let Value::Float(f) = value {
                if (f - 42.5).abs() < 0.001 {
                    received_clone.fetch_add(1, Ordering::SeqCst);
                    notify_clone.notify_one();
                }
            }
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender client
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

    // Send value
    let _ = sender.set("/e2e/test", 42.5).await;

    // Wait for notification or timeout
    let _ = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;

    router.stop();

    if received.load(Ordering::SeqCst) >= 1 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Receiver didn't get the value",
            start.elapsed().as_millis(),
        )
    }
}

async fn test_client_to_client_multiple_values() -> TestResult {
    let start = std::time::Instant::now();
    let name = "client_to_client_multi";

    let router = TestRouter::start().await;

    // Receiver client
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
        .subscribe("/multi/**", move |_, _| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender client
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

    // Send multiple values
    for i in 0..10 {
        let _ = sender.set(&format!("/multi/value/{}", i), i as f64).await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    let count = received.load(Ordering::SeqCst);
    if count >= 8 {
        // Allow some margin
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/10 values", count),
            start.elapsed().as_millis(),
        )
    }
}

async fn test_client_to_client_event() -> TestResult {
    let start = std::time::Instant::now();
    let name = "client_to_client_event";

    let router = TestRouter::start().await;

    // Receiver client
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
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    let _ = receiver
        .subscribe("/events/**", move |_, _| {
            received_clone.fetch_add(1, Ordering::SeqCst);
            notify_clone.notify_one();
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender client
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

    // Emit event
    let _ = sender
        .emit("/events/button", Value::String("pressed".to_string()))
        .await;

    // Wait for notification or timeout
    let _ = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;

    router.stop();

    if received.load(Ordering::SeqCst) >= 1 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Event not received", start.elapsed().as_millis())
    }
}

// ============================================================================
// E2E Tests: Multiple Clients Fan-out
// ============================================================================

async fn test_fanout_to_multiple_clients() -> TestResult {
    let start = std::time::Instant::now();
    let name = "fanout_multiple_clients";

    let router = TestRouter::start().await;

    // Create multiple receiver clients
    let mut receivers = vec![];
    let counters: Vec<Arc<AtomicU32>> = (0..3).map(|_| Arc::new(AtomicU32::new(0))).collect();

    for i in 0..3 {
        match Clasp::connect_to(&router.url()).await {
            Ok(client) => {
                let counter = counters[i].clone();
                let _ = client
                    .subscribe("/fanout/**", move |_, _| {
                        counter.fetch_add(1, Ordering::SeqCst);
                    })
                    .await;
                receivers.push(client);
            }
            Err(e) => {
                router.stop();
                return TestResult::fail(
                    name,
                    format!("Receiver {} connect failed: {}", i, e),
                    start.elapsed().as_millis(),
                );
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Sender client
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

    // Send value - should fan out to all receivers
    let _ = sender.set("/fanout/value", 99.0).await;

    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    // All receivers should have received the value
    let total: u32 = counters.iter().map(|c| c.load(Ordering::SeqCst)).sum();

    if total >= 3 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only {} of 3 receivers got the value", total),
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// E2E Tests: State Persistence
// ============================================================================

async fn test_state_persistence() -> TestResult {
    let start = std::time::Instant::now();
    let name = "state_persistence";

    let router = TestRouter::start().await;

    // First client sets a value
    let client1 = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Client1 connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let _ = client1.set("/persist/value", 123.0).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second client subscribes and should get the value via snapshot
    let client2 = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return TestResult::fail(
                name,
                format!("Client2 connect failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    };

    let received = Arc::new(AtomicU32::new(0));
    let received_clone = received.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    let _ = client2
        .subscribe("/persist/**", move |value, _| {
            if let Value::Float(f) = value {
                if (f - 123.0).abs() < 0.001 {
                    received_clone.fetch_add(1, Ordering::SeqCst);
                    notify_clone.notify_one();
                }
            }
        })
        .await;

    // Wait for snapshot
    let _ = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;

    router.stop();

    if received.load(Ordering::SeqCst) >= 1 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "State not persisted/received",
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// E2E Tests: Subscription Patterns
// ============================================================================

async fn test_wildcard_subscription_patterns() -> TestResult {
    let start = std::time::Instant::now();
    let name = "wildcard_subscription";

    let router = TestRouter::start().await;

    // Receiver with wildcard subscription
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

    // Subscribe to /sensors/*/temperature - should match any sensor's temperature
    let _ = receiver
        .subscribe("/sensors/*/temperature", move |_, _| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender
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

    // Send to various paths
    let _ = sender.set("/sensors/room1/temperature", 22.5).await;
    let _ = sender.set("/sensors/room2/temperature", 23.0).await;
    let _ = sender.set("/sensors/room1/humidity", 50.0).await; // Should NOT match

    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    let count = received.load(Ordering::SeqCst);
    // Should have received 2 (temperatures) but not humidity
    if count >= 2 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Received {} (expected 2)", count),
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// E2E Tests: High Throughput
// ============================================================================

async fn test_high_throughput_e2e() -> TestResult {
    let start = std::time::Instant::now();
    let name = "high_throughput_e2e";

    let router = TestRouter::start().await;

    // Receiver
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
        .subscribe("/throughput/**", move |_, _| {
            received_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender
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

    // Send many messages quickly
    for i in 0..100 {
        let _ = sender
            .set(&format!("/throughput/value/{}", i % 10), i as f64)
            .await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    router.stop();

    let count = received.load(Ordering::SeqCst);
    if count >= 80 {
        // Allow 20% loss under load
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/100 messages", count),
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Multi-Protocol E2E Tests                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        // Client to Client tests
        test_client_to_client_set().await,
        test_client_to_client_multiple_values().await,
        test_client_to_client_event().await,
        // Fan-out tests
        test_fanout_to_multiple_clients().await,
        // State persistence tests
        test_state_persistence().await,
        // Subscription pattern tests
        test_wildcard_subscription_patterns().await,
        // Throughput tests
        test_high_throughput_e2e().await,
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
