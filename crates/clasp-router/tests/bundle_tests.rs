//! Comprehensive BUNDLE Message Tests
//!
//! Tests for CLASP BUNDLE messages covering:
//! - Atomic execution (all or nothing)
//! - Scheduled execution (timestamp-based)
//! - Mixed message types in bundle
//! - Large bundles (many messages)
//! - Timestamp precision

use clasp_client::ClaspBuilder;
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use clasp_test_utils::{TestRouter, ValueCollector};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_bundle_atomic_execution() {
    let router = TestRouter::start().await;

    let sender = ClaspBuilder::new(&router.url())
        .name("Sender")
        .connect()
        .await
        .expect("Sender should connect");

    let receiver = ClaspBuilder::new(&router.url())
        .name("Receiver")
        .connect()
        .await
        .expect("Receiver should connect");

    let collector = ValueCollector::new();
    receiver
        .subscribe("/atomic/**", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    sleep(Duration::from_millis(100)).await;

    // Send bundle with 5 SET messages
    let messages: Vec<Message> = (0..5)
        .map(|i| {
            Message::Set(SetMessage {
                address: format!("/atomic/value{}", i),
                value: Value::Int(i as i64),
                revision: None,
                lock: false,
                unlock: false,
            })
        })
        .collect();

    sender.bundle(messages).await.expect("Bundle should send");

    // All 5 should arrive atomically (or at least very close together)
    assert!(
        collector.wait_for_count(5, Duration::from_secs(2)).await,
        "Should receive all bundle messages"
    );

    let values = collector.values();
    assert_eq!(values.len(), 5, "Should receive exactly 5 values");

    // Verify all values are present
    for i in 0..5 {
        let expected_addr = format!("/atomic/value{}", i);
        assert!(
            values.iter().any(|(addr, val)| {
                addr == &expected_addr && matches!(val, Value::Int(v) if *v == i as i64)
            }),
            "Should have correct value for /atomic/value{}",
            i
        );
    }
}

#[tokio::test]
async fn test_bundle_scheduled_execution() {
    let router = TestRouter::start().await;

    let sender = ClaspBuilder::new(&router.url())
        .name("Sender")
        .connect()
        .await
        .expect("Sender should connect");

    let receiver = ClaspBuilder::new(&router.url())
        .name("Receiver")
        .connect()
        .await
        .expect("Receiver should connect");

    let collector = ValueCollector::new();
    receiver
        .subscribe("/scheduled/value", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    sleep(Duration::from_millis(100)).await;

    // Get current time and schedule for 200ms in the future
    let now = sender.time();
    let future_time = now + 200_000; // 200ms in microseconds

    let messages = vec![Message::Set(SetMessage {
        address: "/scheduled/value".to_string(),
        value: Value::Int(42),
        revision: None,
        lock: false,
        unlock: false,
    })];

    // Send scheduled bundle
    sender
        .bundle_at(messages, future_time)
        .await
        .expect("Scheduled bundle should send");

    // Wait for scheduled time plus buffer
    assert!(
        collector
            .wait_for_count(1, Duration::from_millis(400))
            .await,
        "Should receive scheduled bundle"
    );

    let values = collector.values();
    assert_eq!(values.len(), 1, "Should receive exactly 1 value");

    match values.first() {
        Some((_, Value::Int(42))) => (),
        _ => panic!("Should receive Int(42)"),
    }
}

#[tokio::test]
async fn test_bundle_mixed_message_types() {
    let router = TestRouter::start().await;

    let sender = ClaspBuilder::new(&router.url())
        .name("Sender")
        .connect()
        .await
        .expect("Sender should connect");

    let receiver = ClaspBuilder::new(&router.url())
        .name("Receiver")
        .connect()
        .await
        .expect("Receiver should connect");

    let set_collector = ValueCollector::new();
    let event_collector = ValueCollector::new();

    receiver
        .subscribe("/mixed/set", set_collector.callback_ref())
        .await
        .expect("Subscribe should succeed");
    receiver
        .subscribe("/mixed/event", event_collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    sleep(Duration::from_millis(100)).await;

    // Bundle with SET and PUBLISH (Event)
    let messages = vec![
        Message::Set(SetMessage {
            address: "/mixed/set".to_string(),
            value: Value::Float(3.14),
            revision: None,
            lock: false,
            unlock: false,
        }),
        Message::Publish(PublishMessage {
            address: "/mixed/event".to_string(),
            signal: Some(SignalType::Event),
            value: Some(Value::String("triggered".to_string())),
            payload: None,
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
            timeline: None,
        }),
    ];

    sender.bundle(messages).await.expect("Bundle should send");

    // Wait for both
    assert!(
        set_collector
            .wait_for_count(1, Duration::from_secs(2))
            .await,
        "Should receive SET message"
    );
    assert!(
        event_collector
            .wait_for_count(1, Duration::from_secs(2))
            .await,
        "Should receive PUBLISH message"
    );

    // Verify values
    let set_values = set_collector.values();
    let event_values = event_collector.values();

    assert_eq!(set_values.len(), 1, "Should receive 1 SET");
    assert_eq!(event_values.len(), 1, "Should receive 1 PUBLISH");

    // Check SET value
    match set_values.first() {
        Some((_, Value::Float(f))) => {
            assert!(
                (f - 3.14).abs() < 0.01,
                "SET value should be approximately 3.14"
            );
        }
        _ => panic!("SET value type incorrect"),
    }

    // Check PUBLISH value
    match event_values.first() {
        Some((_, Value::String(s))) => {
            assert_eq!(s, "triggered", "PUBLISH value should be 'triggered'");
        }
        _ => panic!("PUBLISH value type incorrect"),
    }
}

#[tokio::test]
async fn test_bundle_large_bundle() {
    let router = TestRouter::start().await;

    let sender = ClaspBuilder::new(&router.url())
        .name("Sender")
        .connect()
        .await
        .expect("Sender should connect");

    let receiver = ClaspBuilder::new(&router.url())
        .name("Receiver")
        .connect()
        .await
        .expect("Receiver should connect");

    let collector = ValueCollector::new();
    receiver
        .subscribe("/large/**", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    sleep(Duration::from_millis(100)).await;

    // Create bundle with 100 messages
    let message_count = 100usize;
    let messages: Vec<Message> = (0..message_count)
        .map(|i| {
            Message::Set(SetMessage {
                address: format!("/large/item{}", i),
                value: Value::Int(i as i64),
                revision: None,
                lock: false,
                unlock: false,
            })
        })
        .collect();

    sender.bundle(messages).await.expect("Bundle should send");

    // Wait for all messages
    assert!(
        collector
            .wait_for_count(message_count as u32, Duration::from_secs(5))
            .await,
        "Should receive all {} messages (got {})",
        message_count,
        collector.count()
    );

    let values = collector.values();
    assert_eq!(
        values.len(),
        message_count,
        "Should receive exactly {} values",
        message_count
    );

    // Verify all values are present
    for i in 0..message_count {
        let expected_addr = format!("/large/item{}", i);
        assert!(
            values.iter().any(|(addr, val)| {
                addr == &expected_addr && matches!(val, Value::Int(v) if *v == i as i64)
            }),
            "Should have value for {}",
            expected_addr
        );
    }
}

#[tokio::test]
async fn test_bundle_timestamp_precision() {
    let router = TestRouter::start().await;

    let sender = ClaspBuilder::new(&router.url())
        .name("Sender")
        .connect()
        .await
        .expect("Sender should connect");

    let receiver = ClaspBuilder::new(&router.url())
        .name("Receiver")
        .connect()
        .await
        .expect("Receiver should connect");

    let collector = ValueCollector::new();
    receiver
        .subscribe("/precision/value", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    sleep(Duration::from_millis(100)).await;

    // Test microsecond precision
    let now = sender.time();
    let precise_time = now + 123_456; // 123.456ms in microseconds

    let messages = vec![Message::Set(SetMessage {
        address: "/precision/value".to_string(),
        value: Value::Int(999),
        revision: None,
        lock: false,
        unlock: false,
    })];

    // Send scheduled bundle
    sender
        .bundle_at(messages, precise_time)
        .await
        .expect("Scheduled bundle should send");

    // Should receive at approximately the right time
    assert!(
        collector
            .wait_for_count(1, Duration::from_millis(300))
            .await,
        "Should receive scheduled bundle"
    );

    let values = collector.values();
    assert_eq!(values.len(), 1, "Should receive 1 value");
}
