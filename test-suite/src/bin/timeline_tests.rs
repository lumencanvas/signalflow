//! Timeline Signal Type Tests
//!
//! End-to-end tests for timeline signals, verifying:
//! - Timeline publishing and routing
//! - TimelinePlayer interpolation
//! - Easing functions
//! - Looping behavior
//! - Pause/resume functionality

use clasp_client::Clasp;
use clasp_core::{
    timeline::{PlaybackState, TimelinePlayer},
    EasingType, SecurityMode, SignalType, TimelineData, TimelineKeyframe, Value,
};
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
            name: "Timeline Test Router".to_string(),
            max_sessions: 100,
            session_timeout: 60,
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
                "timeline".to_string(),
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
// Timeline Player Unit Tests (using E2E framework for consistency)
// ============================================================================

/// Test: Linear interpolation
async fn test_timeline_linear() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_linear";

    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::Linear,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000, // 1 second in microseconds
            value: Value::Float(100.0),
            easing: EasingType::Linear,
            bezier: None,
        },
    ]);

    let mut player = TimelinePlayer::new(timeline);
    player.start(0);

    // Test at t=0
    let val = player.sample(0).unwrap();
    let v0 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float at t=0", start.elapsed().as_millis()),
    };
    if (v0 - 0.0).abs() > 0.1 {
        return TestResult::fail(
            name,
            format!("Expected 0.0 at t=0, got {}", v0),
            start.elapsed().as_millis(),
        );
    }

    // Test at t=0.5s
    let val = player.sample(500_000).unwrap();
    let v50 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float at t=0.5", start.elapsed().as_millis()),
    };
    if (v50 - 50.0).abs() > 0.1 {
        return TestResult::fail(
            name,
            format!("Expected 50.0 at t=0.5, got {}", v50),
            start.elapsed().as_millis(),
        );
    }

    // Test at t=1s
    let val = player.sample(1_000_000).unwrap();
    let v100 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float at t=1", start.elapsed().as_millis()),
    };
    if (v100 - 100.0).abs() > 0.1 {
        return TestResult::fail(
            name,
            format!("Expected 100.0 at t=1, got {}", v100),
            start.elapsed().as_millis(),
        );
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

/// Test: EaseIn curve
async fn test_timeline_ease_in() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_ease_in";

    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::EaseIn,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000,
            value: Value::Float(100.0),
            easing: EasingType::Linear,
            bezier: None,
        },
    ]);

    let mut player = TimelinePlayer::new(timeline);
    player.start(0);

    // At t=0.5, ease-in should be less than 50 (slower start)
    let val = player.sample(500_000).unwrap();
    let v50 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float", start.elapsed().as_millis()),
    };

    if v50 >= 50.0 {
        return TestResult::fail(
            name,
            format!("EaseIn at t=0.5 should be < 50, got {}", v50),
            start.elapsed().as_millis(),
        );
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

/// Test: EaseOut curve
async fn test_timeline_ease_out() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_ease_out";

    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::EaseOut,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000,
            value: Value::Float(100.0),
            easing: EasingType::Linear,
            bezier: None,
        },
    ]);

    let mut player = TimelinePlayer::new(timeline);
    player.start(0);

    // At t=0.5, ease-out should be more than 50 (faster start)
    let val = player.sample(500_000).unwrap();
    let v50 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float", start.elapsed().as_millis()),
    };

    if v50 <= 50.0 {
        return TestResult::fail(
            name,
            format!("EaseOut at t=0.5 should be > 50, got {}", v50),
            start.elapsed().as_millis(),
        );
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

/// Test: Looping timeline
async fn test_timeline_loop() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_loop";

    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::Linear,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000,
            value: Value::Float(100.0),
            easing: EasingType::Linear,
            bezier: None,
        },
    ])
    .with_loop(true);

    let mut player = TimelinePlayer::new(timeline);
    player.start(0);

    // First loop at t=0.5s
    let _ = player.sample(500_000);

    // Second loop at t=1.5s (500ms into second loop)
    let val = player.sample(1_500_000).unwrap();
    let v = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float", start.elapsed().as_millis()),
    };

    if (v - 50.0).abs() > 1.0 {
        return TestResult::fail(
            name,
            format!("Second loop at t=1.5s should be ~50, got {}", v),
            start.elapsed().as_millis(),
        );
    }

    if player.loop_count() < 1 {
        return TestResult::fail(name, "Expected loop_count >= 1", start.elapsed().as_millis());
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

/// Test: Timeline finished state
async fn test_timeline_finished() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_finished";

    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::Linear,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000,
            value: Value::Float(100.0),
            easing: EasingType::Linear,
            bezier: None,
        },
    ]);

    let mut player = TimelinePlayer::new(timeline);
    player.start(0);

    // Play to end
    let _ = player.sample(2_000_000);

    if player.state() != PlaybackState::Finished {
        return TestResult::fail(
            name,
            format!("Expected Finished state, got {:?}", player.state()),
            start.elapsed().as_millis(),
        );
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

/// Test: Timeline routing through server
async fn test_timeline_routing() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_routing";

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

    let received_count = Arc::new(AtomicU32::new(0));
    let counter = received_count.clone();

    let _ = receiver
        .subscribe("/timeline/**", move |_value, _address| {
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

    // Create and send a timeline
    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::Linear,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000,
            value: Value::Float(1.0),
            easing: EasingType::EaseOut,
            bezier: None,
        },
    ]);

    if let Err(e) = sender.timeline("/timeline/dimmer", timeline).await {
        router.stop();
        return TestResult::fail(
            name,
            format!("Timeline send failed: {}", e),
            start.elapsed().as_millis(),
        );
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    router.stop();

    let count = received_count.load(Ordering::SeqCst);
    if count >= 1 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Expected >= 1 timeline message, got {}", count),
            start.elapsed().as_millis(),
        )
    }
}

/// Test: Multiple keyframes interpolation
async fn test_timeline_multi_keyframe() -> TestResult {
    let start = std::time::Instant::now();
    let name = "timeline_multi_keyframe";

    let timeline = TimelineData::new(vec![
        TimelineKeyframe {
            time: 0,
            value: Value::Float(0.0),
            easing: EasingType::Linear,
            bezier: None,
        },
        TimelineKeyframe {
            time: 500_000, // 0.5s
            value: Value::Float(100.0),
            easing: EasingType::Linear,
            bezier: None,
        },
        TimelineKeyframe {
            time: 1_000_000, // 1s
            value: Value::Float(50.0),
            easing: EasingType::Linear,
            bezier: None,
        },
    ]);

    let mut player = TimelinePlayer::new(timeline);
    player.start(0);

    // At t=0.25s, should be around 50 (halfway to first keyframe)
    let val = player.sample(250_000).unwrap();
    let v25 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float at t=0.25", start.elapsed().as_millis()),
    };
    if (v25 - 50.0).abs() > 1.0 {
        return TestResult::fail(
            name,
            format!("Expected ~50 at t=0.25s, got {}", v25),
            start.elapsed().as_millis(),
        );
    }

    // At t=0.75s, should be around 75 (halfway from 100 to 50)
    let val = player.sample(750_000).unwrap();
    let v75 = match val {
        Value::Float(v) => v,
        _ => return TestResult::fail(name, "Expected float at t=0.75", start.elapsed().as_millis()),
    };
    if (v75 - 75.0).abs() > 1.0 {
        return TestResult::fail(
            name,
            format!("Expected ~75 at t=0.75s, got {}", v75),
            start.elapsed().as_millis(),
        );
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                   CLASP Timeline Signal Tests                    ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        test_timeline_linear().await,
        test_timeline_ease_in().await,
        test_timeline_ease_out().await,
        test_timeline_loop().await,
        test_timeline_finished().await,
        test_timeline_routing().await,
        test_timeline_multi_keyframe().await,
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
