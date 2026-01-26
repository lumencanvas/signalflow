//! Timeline Signal Type Tests
//!
//! Tests for timeline signals, verifying:
//! - Timeline publishing and routing
//! - TimelinePlayer interpolation
//! - Easing functions
//! - Looping behavior
//! - Pause/resume functionality

use clasp_core::{
    timeline::{PlaybackState, TimelinePlayer},
    EasingType, SecurityMode, TimelineData, TimelineKeyframe, Value,
};
use clasp_router::RouterConfig;
use clasp_test_utils::{TestRouter, ValueCollector};
use std::time::Duration;

// ============================================================================
// Timeline Player Unit Tests
// ============================================================================

/// Test: Linear interpolation
#[tokio::test]
async fn test_timeline_linear() {
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
        _ => panic!("Expected float at t=0"),
    };
    assert!((v0 - 0.0).abs() <= 0.1, "Expected 0.0 at t=0, got {}", v0);

    // Test at t=0.5s
    let val = player.sample(500_000).unwrap();
    let v50 = match val {
        Value::Float(v) => v,
        _ => panic!("Expected float at t=0.5"),
    };
    assert!(
        (v50 - 50.0).abs() <= 0.1,
        "Expected 50.0 at t=0.5, got {}",
        v50
    );

    // Test at t=1s
    let val = player.sample(1_000_000).unwrap();
    let v100 = match val {
        Value::Float(v) => v,
        _ => panic!("Expected float at t=1"),
    };
    assert!(
        (v100 - 100.0).abs() <= 0.1,
        "Expected 100.0 at t=1, got {}",
        v100
    );
}

/// Test: EaseIn curve
#[tokio::test]
async fn test_timeline_ease_in() {
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
        _ => panic!("Expected float"),
    };

    assert!(v50 < 50.0, "EaseIn at t=0.5 should be < 50, got {}", v50);
}

/// Test: EaseOut curve
#[tokio::test]
async fn test_timeline_ease_out() {
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
        _ => panic!("Expected float"),
    };

    assert!(v50 > 50.0, "EaseOut at t=0.5 should be > 50, got {}", v50);
}

/// Test: Looping timeline
#[tokio::test]
async fn test_timeline_loop() {
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
        _ => panic!("Expected float"),
    };

    assert!(
        (v - 50.0).abs() <= 1.0,
        "Second loop at t=1.5s should be ~50, got {}",
        v
    );

    assert!(
        player.loop_count() >= 1,
        "Expected loop_count >= 1, got {}",
        player.loop_count()
    );
}

/// Test: Timeline finished state
#[tokio::test]
async fn test_timeline_finished() {
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

    assert_eq!(
        player.state(),
        PlaybackState::Finished,
        "Expected Finished state, got {:?}",
        player.state()
    );
}

/// Test: Timeline routing through server
#[tokio::test]
async fn test_timeline_routing() {
    let router = TestRouter::start_with_config(RouterConfig {
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
        gesture_coalesce_interval_ms: 0,
        max_messages_per_second: 0,
        rate_limiting_enabled: false,
        ..Default::default()
    })
    .await;

    // Receiver
    let receiver = router
        .connect_client()
        .await
        .expect("Receiver should connect");

    let collector = ValueCollector::new();
    receiver
        .subscribe("/timeline/**", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender
    let sender = router
        .connect_client()
        .await
        .expect("Sender should connect");

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

    sender
        .timeline("/timeline/dimmer", timeline)
        .await
        .expect("Timeline send should succeed");

    // Wait for message to be received
    assert!(
        collector.wait_for_count(1, Duration::from_secs(2)).await,
        "Expected >= 1 timeline message, got {}",
        collector.count()
    );
}

/// Test: Multiple keyframes interpolation
#[tokio::test]
async fn test_timeline_multi_keyframe() {
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
        _ => panic!("Expected float at t=0.25"),
    };
    assert!(
        (v25 - 50.0).abs() <= 1.0,
        "Expected ~50 at t=0.25s, got {}",
        v25
    );

    // At t=0.75s, should be around 75 (halfway from 100 to 50)
    let val = player.sample(750_000).unwrap();
    let v75 = match val {
        Value::Float(v) => v,
        _ => panic!("Expected float at t=0.75"),
    };
    assert!(
        (v75 - 75.0).abs() <= 1.0,
        "Expected ~75 at t=0.75s, got {}",
        v75
    );
}
