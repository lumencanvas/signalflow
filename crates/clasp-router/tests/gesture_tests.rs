//! Gesture Signal Type Tests
//!
//! End-to-end tests for gesture signals, verifying:
//! - Start -> Move -> End lifecycle
//! - Multiple concurrent gestures (different IDs)
//! - Gesture cancellation
//! - High-frequency move updates
//! - Cross-client gesture routing

use clasp_client::Clasp;
use clasp_core::{GesturePhase, Value};
use clasp_test_utils::TestRouter;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Test: Single gesture lifecycle (Start -> Move -> End)
#[tokio::test]
async fn test_gesture_lifecycle() {
    let router = TestRouter::start().await;

    // Receiver client
    let receiver = Clasp::connect_to(&router.url())
        .await
        .expect("Receiver connect failed");

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
    let sender = Clasp::connect_to(&router.url())
        .await
        .expect("Sender connect failed");

    // Send gesture lifecycle
    let gesture_id = 1u32;

    // Start
    sender
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
        .expect("Gesture start failed");

    // Move (multiple)
    for i in 0..5 {
        let x = 0.5 + (i as f64 * 0.1);
        let y = 0.3 + (i as f64 * 0.05);
        sender
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
            .expect("Gesture move failed");
    }

    // End
    sender
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
        .expect("Gesture end failed");

    // Wait for messages to arrive
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check that we received the gestures
    // With move coalescing enabled: Start + 1 coalesced Move + End = 3 messages
    // Without coalescing: Start + 5 moves + End = 7 messages
    // We expect at least 3 (coalesced) since router has coalescing enabled
    let count = gesture_count.load(Ordering::SeqCst);
    assert!(
        count >= 3,
        "Only received {} gesture messages, expected >= 3",
        count
    );
}

/// Test: Multiple concurrent gestures (multitouch)
#[tokio::test]
async fn test_multitouch_gestures() {
    let router = TestRouter::start().await;

    // Receiver
    let receiver = Clasp::connect_to(&router.url())
        .await
        .expect("Receiver connect failed");

    let gesture_count = Arc::new(AtomicU32::new(0));
    let counter = gesture_count.clone();

    let _ = receiver
        .subscribe("/multitouch/**", move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender
    let sender = Clasp::connect_to(&router.url())
        .await
        .expect("Sender connect failed");

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

    let count = gesture_count.load(Ordering::SeqCst);
    // 3 touches * 3 phases = 9 gestures
    assert!(count >= 8, "Only received {}/9 gesture messages", count);
}

/// Test: Gesture cancellation
#[tokio::test]
async fn test_gesture_cancel() {
    let router = TestRouter::start().await;

    // Receiver
    let receiver = Clasp::connect_to(&router.url())
        .await
        .expect("Receiver connect failed");

    let received_cancel = Arc::new(AtomicU32::new(0));
    let cancel_counter = received_cancel.clone();

    let _ = receiver
        .subscribe("/cancel/**", move |_, _| {
            cancel_counter.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender
    let sender = Clasp::connect_to(&router.url())
        .await
        .expect("Sender connect failed");

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

    let count = received_cancel.load(Ordering::SeqCst);
    assert!(count >= 3, "Only received {}/3 messages", count);
}

/// Test: High-frequency gesture moves
#[tokio::test]
async fn test_gesture_high_frequency() {
    let router = TestRouter::start().await;

    // Receiver
    let receiver = Clasp::connect_to(&router.url())
        .await
        .expect("Receiver connect failed");

    let move_count = Arc::new(AtomicU32::new(0));
    let counter = move_count.clone();

    let _ = receiver
        .subscribe("/highfreq/**", move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Sender
    let sender = Clasp::connect_to(&router.url())
        .await
        .expect("Sender connect failed");

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

    let count = move_count.load(Ordering::SeqCst);
    // With gesture coalescing enabled (16ms interval):
    // - We expect Start + some coalesced Moves + End
    // - At 60Hz, with 16ms coalesce window, we get ~1-2 moves per flush
    // - Minimum: Start (1) + 1 coalesced Move + End (1) = 3
    // - The key is that coalescing reduces the message count significantly
    // Without coalescing: 102 messages
    // With coalescing: typically 3-20 depending on timing
    assert!(
        count >= 3,
        "Only received {}/102 messages (expected >= 3 with coalescing)",
        count
    );
}
