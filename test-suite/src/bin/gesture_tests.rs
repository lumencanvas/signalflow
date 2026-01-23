//! Gesture Signal Type Tests
//!
//! End-to-end tests for gesture signals, verifying:
//! - Start → Move → End lifecycle
//! - Multiple concurrent gestures (different IDs)
//! - Gesture cancellation
//! - High-frequency move updates
//! - Cross-client gesture routing

use clasp_client::Clasp;
use clasp_core::{GesturePhase, SecurityMode, Value};
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

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
            name: "Gesture Test Router".to_string(),
            max_sessions: 100,
            session_timeout: 60,
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
                "gesture".to_string(),
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
// Gesture E2E Tests
// ============================================================================

/// Test: Single gesture lifecycle (Start → Move → End)
async fn test_gesture_lifecycle() -> TestResult {
    let start = std::time::Instant::now();
    let name = "gesture_lifecycle";

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

    // Track received gestures by phase
    let gesture_count = Arc::new(AtomicU32::new(0));
    let counter = gesture_count.clone();

    let _ = receiver
        .subscribe("/input/**", move |_value, _address| {
            // Track that we got a gesture message
            counter.fetch_add(1, Ordering::SeqCst);
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

    // Send gesture lifecycle
    let gesture_id = 1u32;

    // Start
    if let Err(e) = sender
        .gesture(
            "/input/touch",
            gesture_id,
            GesturePhase::Start,
            Value::Map(
                vec![
                    ("x".to_string(), Value::Float(0.5)),
                    ("y".to_string(), Value::Float(0.3)),
                ]
                .into_iter()
                .collect(),
            ),
        )
        .await
    {
        router.stop();
        return TestResult::fail(
            name,
            format!("Gesture start failed: {}", e),
            start.elapsed().as_millis(),
        );
    }

    // Move (multiple)
    for i in 0..5 {
        let x = 0.5 + (i as f64 * 0.1);
        let y = 0.3 + (i as f64 * 0.05);
        if let Err(e) = sender
            .gesture(
                "/input/touch",
                gesture_id,
                GesturePhase::Move,
                Value::Map(
                    vec![
                        ("x".to_string(), Value::Float(x)),
                        ("y".to_string(), Value::Float(y)),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )
            .await
        {
            router.stop();
            return TestResult::fail(
                name,
                format!("Gesture move failed: {}", e),
                start.elapsed().as_millis(),
            );
        }
    }

    // End
    if let Err(e) = sender
        .gesture(
            "/input/touch",
            gesture_id,
            GesturePhase::End,
            Value::Map(
                vec![
                    ("x".to_string(), Value::Float(1.0)),
                    ("y".to_string(), Value::Float(0.55)),
                ]
                .into_iter()
                .collect(),
            ),
        )
        .await
    {
        router.stop();
        return TestResult::fail(
            name,
            format!("Gesture end failed: {}", e),
            start.elapsed().as_millis(),
        );
    }

    // Wait for messages to arrive
    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    // Check that we received the gestures
    // With move coalescing enabled: Start + 1 coalesced Move + End = 3 messages
    // Without coalescing: Start + 5 moves + End = 7 messages
    // We expect at least 3 (coalesced) since router has coalescing enabled
    let count = gesture_count.load(Ordering::SeqCst);
    if count >= 3 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only received {} gesture messages, expected >= 3", count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test: Multiple concurrent gestures (multitouch)
async fn test_multitouch_gestures() -> TestResult {
    let start = std::time::Instant::now();
    let name = "multitouch_gestures";

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

    let gesture_count = Arc::new(AtomicU32::new(0));
    let counter = gesture_count.clone();

    let _ = receiver
        .subscribe("/multitouch/**", move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
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

    // Send 3 concurrent touch gestures
    for touch_id in 0u32..3 {
        // Start
        let _ = sender
            .gesture(
                "/multitouch/finger",
                touch_id,
                GesturePhase::Start,
                Value::Int(touch_id as i64),
            )
            .await;

        // Move
        let _ = sender
            .gesture(
                "/multitouch/finger",
                touch_id,
                GesturePhase::Move,
                Value::Int(touch_id as i64),
            )
            .await;

        // End
        let _ = sender
            .gesture(
                "/multitouch/finger",
                touch_id,
                GesturePhase::End,
                Value::Int(touch_id as i64),
            )
            .await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    let count = gesture_count.load(Ordering::SeqCst);
    // 3 touches * 3 phases = 9 gestures
    if count >= 8 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/9 gesture messages", count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test: Gesture cancellation
async fn test_gesture_cancel() -> TestResult {
    let start = std::time::Instant::now();
    let name = "gesture_cancel";

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

    let received_cancel = Arc::new(AtomicU32::new(0));
    let cancel_counter = received_cancel.clone();

    let _ = receiver
        .subscribe("/cancel/**", move |_, _| {
            cancel_counter.fetch_add(1, Ordering::SeqCst);
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

    // Start gesture, then cancel (simulating interrupted touch)
    let _ = sender
        .gesture("/cancel/touch", 1, GesturePhase::Start, Value::Null)
        .await;
    let _ = sender
        .gesture("/cancel/touch", 1, GesturePhase::Move, Value::Null)
        .await;
    let _ = sender
        .gesture("/cancel/touch", 1, GesturePhase::Cancel, Value::Null)
        .await;

    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    let count = received_cancel.load(Ordering::SeqCst);
    if count >= 3 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/3 messages", count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test: High-frequency gesture moves
async fn test_gesture_high_frequency() -> TestResult {
    let start = std::time::Instant::now();
    let name = "gesture_high_frequency";

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

    let move_count = Arc::new(AtomicU32::new(0));
    let counter = move_count.clone();

    let _ = receiver
        .subscribe("/highfreq/**", move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
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

    // Start
    let _ = sender
        .gesture("/highfreq/pen", 1, GesturePhase::Start, Value::Float(0.0))
        .await;

    // Send 100 move updates (simulating 60Hz pen input)
    for i in 0..100 {
        let _ = sender
            .gesture(
                "/highfreq/pen",
                1,
                GesturePhase::Move,
                Value::Float(i as f64 / 100.0),
            )
            .await;
    }

    // End
    let _ = sender
        .gesture("/highfreq/pen", 1, GesturePhase::End, Value::Float(1.0))
        .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    router.stop();

    let count = move_count.load(Ordering::SeqCst);
    // With gesture coalescing enabled (16ms interval):
    // - We expect Start + some coalesced Moves + End
    // - At 60Hz, with 16ms coalesce window, we get ~1-2 moves per flush
    // - Minimum: Start (1) + 1 coalesced Move + End (1) = 3
    // - The key is that coalescing reduces the message count significantly
    // Without coalescing: 102 messages
    // With coalescing: typically 3-20 depending on timing
    if count >= 3 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Only received {}/102 messages (expected >= 3 with coalescing)", count),
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
    println!("║                   CLASP Gesture Signal Tests                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_gesture_lifecycle().await,
        test_multitouch_gestures().await,
        test_gesture_cancel().await,
        test_gesture_high_frequency().await,
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
