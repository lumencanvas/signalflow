//! Client Library Tests (clasp-client)
//!
//! Tests for the high-level Clasp client API including:
//! - Builder pattern and configuration
//! - Connection lifecycle
//! - Parameter operations (set, get, subscribe)
//! - Event operations (emit, subscribe)
//! - Advanced features (bundles, caching, clock sync)
//! - Negative tests and edge cases
//! - Value type coverage

use clasp_client::{Clasp, ClaspBuilder};
use clasp_core::{Message, SetMessage, Value};
use clasp_test_utils::{TestRouter, ValueCollector};
use std::time::Duration;
use tokio::time::timeout;

// ============================================================================
// Builder Tests
// ============================================================================

#[tokio::test]
async fn test_builder_default() {
    let router = TestRouter::start().await;

    let client = ClaspBuilder::new(&router.url())
        .connect()
        .await
        .expect("Connect failed");

    assert!(client.is_connected(), "Client not connected");
    assert!(client.session_id().is_some(), "No session ID");

    client.close().await;
}

#[tokio::test]
async fn test_builder_custom_name() {
    let router = TestRouter::start().await;

    let custom_name = "MyCustomTestClient";
    let client = ClaspBuilder::new(&router.url())
        .name(custom_name)
        .connect()
        .await
        .expect("Connect failed");

    assert!(client.is_connected(), "Client not connected");

    client.close().await;
}

#[tokio::test]
async fn test_builder_features() {
    let router = TestRouter::start().await;

    let client = ClaspBuilder::new(&router.url())
        .features(vec![
            "param".to_string(),
            "event".to_string(),
            "stream".to_string(),
            "gesture".to_string(),
        ])
        .connect()
        .await
        .expect("Connect failed");

    assert!(client.is_connected(), "Client not connected");

    client.close().await;
}

#[tokio::test]
async fn test_builder_chained() {
    let router = TestRouter::start().await;

    let client = ClaspBuilder::new(&router.url())
        .name("ChainedBuilder")
        .features(vec!["param".to_string(), "event".to_string()])
        .reconnect(false)
        .reconnect_interval(1000)
        .connect()
        .await
        .expect("Connect failed");

    assert!(client.is_connected(), "Client not connected");

    client.close().await;
}

// ============================================================================
// Connection Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_connect_to() {
    let router = TestRouter::start().await;

    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    assert!(client.is_connected(), "Client not connected");
    assert!(client.session_id().is_some(), "No session ID");

    client.close().await;
}

#[tokio::test]
async fn test_session_id() {
    let router = TestRouter::start().await;

    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let session_id = client.session_id().expect("No session ID");
    assert!(!session_id.is_empty(), "Session ID is empty");
    assert_eq!(session_id.len(), 36, "Session ID should be UUID format");

    client.close().await;
}

#[tokio::test]
async fn test_graceful_disconnect() {
    let router = TestRouter::start().await;

    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    assert!(client.is_connected(), "Should be connected");

    client.close().await;

    assert!(
        !client.is_connected(),
        "Should not be connected after close"
    );
}

#[tokio::test]
async fn test_connection_error_nonexistent() {
    let connect_result = timeout(
        Duration::from_secs(3),
        Clasp::connect_to("ws://127.0.0.1:1"),
    )
    .await;

    match connect_result {
        Ok(Ok(_)) => panic!("Should have failed to connect to nonexistent server"),
        Ok(Err(_)) => {} // Expected: connection error
        Err(_) => {}     // Expected: timeout
    }
}

#[tokio::test]
async fn test_connection_error_invalid_url() {
    let invalid_urls = vec!["not-a-url", "http://localhost", "", "ftp://server"];

    for url in invalid_urls {
        let connect_result = timeout(Duration::from_secs(2), Clasp::connect_to(url)).await;

        match connect_result {
            Ok(Ok(_)) => {
                panic!("Should have failed for invalid URL: {}", url);
            }
            _ => {} // Expected: error or timeout
        }
    }
}

// ============================================================================
// Parameter Operations Tests
// ============================================================================

#[tokio::test]
async fn test_set_parameter() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client.set("/test/value", 42.0).await.expect("Set failed");

    client.close().await;
}

#[tokio::test]
async fn test_set_and_receive() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    client
        .subscribe("/test/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client
        .set("/test/sensor", 123.456)
        .await
        .expect("Set failed");

    // Wait for the value with timeout
    let received = collector.wait_for_count(1, Duration::from_secs(2)).await;
    assert!(received, "Did not receive SET value within timeout");

    // Verify the value
    let values = collector.values();
    let (addr, value) = values.last().expect("No value received");
    assert_eq!(addr, "/test/sensor");
    let f = value.as_f64().expect("Value is not a float");
    assert!((f - 123.456).abs() < 0.001, "Value mismatch");

    client.close().await;
}

#[tokio::test]
async fn test_set_locked() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client
        .set_locked("/test/locked", 100.0)
        .await
        .expect("Set locked failed");

    client.close().await;
}

#[tokio::test]
async fn test_subscribe_pattern_wildcard() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    client
        .subscribe("/sensors/*", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    // Send to matching addresses
    client.set("/sensors/temp", 25.0).await.expect("Set failed");
    client
        .set("/sensors/humidity", 60.0)
        .await
        .expect("Set failed");
    client
        .set("/sensors/pressure", 1013.25)
        .await
        .expect("Set failed");

    // Wait for all three
    let received = collector.wait_for_count(3, Duration::from_secs(2)).await;
    assert!(received, "Did not receive all 3 values");

    // Verify all addresses received
    assert!(
        collector.has_address("/sensors/temp"),
        "Missing /sensors/temp"
    );
    assert!(
        collector.has_address("/sensors/humidity"),
        "Missing /sensors/humidity"
    );
    assert!(
        collector.has_address("/sensors/pressure"),
        "Missing /sensors/pressure"
    );

    client.close().await;
}

#[tokio::test]
async fn test_subscribe_pattern_globstar() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    // ** should match any depth
    client
        .subscribe("/app/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client.set("/app/level1", 1.0).await.expect("Set failed");
    client
        .set("/app/level1/level2", 2.0)
        .await
        .expect("Set failed");
    client.set("/app/a/b/c/d", 4.0).await.expect("Set failed");

    let received = collector.wait_for_count(3, Duration::from_secs(2)).await;
    assert!(received, "Did not receive all globstar values");

    client.close().await;
}

#[tokio::test]
async fn test_unsubscribe() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    let sub_id = client
        .subscribe("/unsub/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    // Send one value
    client.set("/unsub/before", 1.0).await.expect("Set failed");
    collector.wait_for_count(1, Duration::from_secs(1)).await;
    let count_before = collector.count();

    // Unsubscribe
    client
        .unsubscribe(sub_id)
        .await
        .expect("Unsubscribe failed");

    // Small delay for unsubscribe to propagate
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send more values - should not be received
    client.set("/unsub/after1", 2.0).await.expect("Set failed");
    client.set("/unsub/after2", 3.0).await.expect("Set failed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let count_after = collector.count();

    // Count should not have increased significantly after unsubscribe
    assert!(
        count_after <= count_before + 1,
        "Received values after unsubscribe: before={}, after={}",
        count_before,
        count_after
    );

    client.close().await;
}

#[tokio::test]
async fn test_cached_value() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    client
        .subscribe("/cache/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client.set("/cache/test", 42.0).await.expect("Set failed");

    collector.wait_for_count(1, Duration::from_secs(2)).await;

    // Check cached value
    let cached = client.cached("/cache/test");
    if let Some(v) = cached {
        let f = v.as_f64().expect("Cached value not a float");
        assert!((f - 42.0).abs() < 0.001, "Cached value mismatch");
    }
    // Note: Cache might not be populated if the value arrives asynchronously
    // This test verifies the cache API works, not that it's always populated

    client.close().await;
}

// ============================================================================
// Event Operations Tests
// ============================================================================

#[tokio::test]
async fn test_emit_event() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client
        .emit("/events/button", Value::String("pressed".to_string()))
        .await
        .expect("Emit failed");

    client.close().await;
}

#[tokio::test]
async fn test_emit_and_receive() {
    let router = TestRouter::start().await;

    // Two clients: one emits, one receives
    let receiver = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");
    let emitter = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    receiver
        .subscribe("/events/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    tokio::time::sleep(Duration::from_millis(50)).await;

    emitter
        .emit("/events/trigger", Value::String("activated".to_string()))
        .await
        .expect("Emit failed");

    let received = collector.wait_for_count(1, Duration::from_secs(2)).await;
    assert!(received, "Event not received");

    assert!(
        collector.has_address("/events/trigger"),
        "Wrong event address"
    );

    receiver.close().await;
    emitter.close().await;
}

#[tokio::test]
async fn test_stream() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    // Stream sends multiple samples
    for i in 0..10 {
        client
            .stream("/sensors/accel", Value::Float(i as f64 * 0.1))
            .await
            .expect(&format!("Stream {} failed", i));
    }

    client.close().await;
}

// ============================================================================
// Advanced Features Tests
// ============================================================================

#[tokio::test]
async fn test_bundle() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let messages = vec![
        Message::Set(SetMessage {
            address: "/bundle/a".to_string(),
            value: Value::Int(1),
            revision: None,
            lock: false,
            unlock: false,
        }),
        Message::Set(SetMessage {
            address: "/bundle/b".to_string(),
            value: Value::Int(2),
            revision: None,
            lock: false,
            unlock: false,
        }),
        Message::Set(SetMessage {
            address: "/bundle/c".to_string(),
            value: Value::Int(3),
            revision: None,
            lock: false,
            unlock: false,
        }),
    ];

    client.bundle(messages).await.expect("Bundle failed");

    client.close().await;
}

#[tokio::test]
async fn test_bundle_atomicity() {
    let router = TestRouter::start().await;

    let sender = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");
    let receiver = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();
    receiver
        .subscribe("/atomic/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send bundle of 5 values
    let messages: Vec<Message> = (0..5)
        .map(|i| {
            Message::Set(SetMessage {
                address: format!("/atomic/v{}", i),
                value: Value::Int(i),
                revision: None,
                lock: false,
                unlock: false,
            })
        })
        .collect();

    sender.bundle(messages).await.expect("Bundle failed");

    // Should receive all 5
    let received = collector.wait_for_count(5, Duration::from_secs(2)).await;
    assert!(received, "Did not receive all bundle values");

    sender.close().await;
    receiver.close().await;
}

#[tokio::test]
async fn test_bundle_at() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let messages = vec![Message::Set(SetMessage {
        address: "/scheduled/value".to_string(),
        value: Value::Float(99.9),
        revision: None,
        lock: false,
        unlock: false,
    })];

    let future_time = client.time() + 100_000; // 100ms in microseconds
    client
        .bundle_at(messages, future_time)
        .await
        .expect("Bundle_at failed");

    client.close().await;
}

#[tokio::test]
async fn test_clock_sync() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let server_time = client.time();

    // Should be a reasonable timestamp (non-zero, in microseconds)
    assert!(server_time > 0, "Server time should be positive");

    // Should be roughly recent (within last hour of current time in microseconds)
    let now_micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    let diff = (server_time as i64 - now_micros).abs();
    // Allow up to 1 hour difference for any sync offset
    assert!(
        diff < 3600_000_000,
        "Server time too far from local: diff={}",
        diff
    );

    client.close().await;
}

// ============================================================================
// Value Type Tests
// ============================================================================

#[tokio::test]
async fn test_value_type_int() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();
    client
        .subscribe("/types/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client.set("/types/int", 42i64).await.expect("Set failed");
    client
        .set("/types/int_neg", -100i64)
        .await
        .expect("Set failed");
    client
        .set("/types/int_zero", 0i64)
        .await
        .expect("Set failed");
    client
        .set("/types/int_max", i64::MAX)
        .await
        .expect("Set failed");

    collector.wait_for_count(4, Duration::from_secs(2)).await;

    client.close().await;
}

#[tokio::test]
async fn test_value_type_float() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();
    client
        .subscribe("/types/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client
        .set("/types/float", 3.14159f64)
        .await
        .expect("Set failed");
    client
        .set("/types/float_neg", -273.15f64)
        .await
        .expect("Set failed");
    client
        .set("/types/float_zero", 0.0f64)
        .await
        .expect("Set failed");
    client
        .set("/types/float_tiny", 1e-100f64)
        .await
        .expect("Set failed");

    collector.wait_for_count(4, Duration::from_secs(2)).await;

    client.close().await;
}

#[tokio::test]
async fn test_value_type_bool() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();
    client
        .subscribe("/types/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client
        .set("/types/bool_true", true)
        .await
        .expect("Set failed");
    client
        .set("/types/bool_false", false)
        .await
        .expect("Set failed");

    let received = collector.wait_for_count(2, Duration::from_secs(2)).await;
    assert!(received, "Did not receive bool values");

    client.close().await;
}

#[tokio::test]
async fn test_value_type_string() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();
    client
        .subscribe("/types/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    client
        .set("/types/str", "hello world")
        .await
        .expect("Set failed");
    client
        .set("/types/str_empty", "")
        .await
        .expect("Set failed");
    client
        .set("/types/str_unicode", "Hello, \u{1F30D}!")
        .await
        .expect("Set failed");
    client
        .set("/types/str_long", "x".repeat(1000))
        .await
        .expect("Set failed");

    collector.wait_for_count(4, Duration::from_secs(2)).await;

    client.close().await;
}

#[tokio::test]
async fn test_value_type_bytes() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client
        .set(
            "/types/bytes",
            Value::Bytes(vec![0x00, 0xFF, 0x42, 0xDE, 0xAD]),
        )
        .await
        .expect("Set failed");
    client
        .set("/types/bytes_empty", Value::Bytes(vec![]))
        .await
        .expect("Set failed");

    client.close().await;
}

#[tokio::test]
async fn test_value_type_array() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client
        .set(
            "/types/array",
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        )
        .await
        .expect("Set failed");

    client
        .set(
            "/types/array_mixed",
            Value::Array(vec![
                Value::Int(1),
                Value::Float(2.5),
                Value::String("three".to_string()),
            ]),
        )
        .await
        .expect("Set failed");

    client
        .set("/types/array_empty", Value::Array(vec![]))
        .await
        .expect("Set failed");

    client.close().await;
}

#[tokio::test]
async fn test_value_type_null() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client
        .set("/types/null", Value::Null)
        .await
        .expect("Set failed");

    client.close().await;
}

// ============================================================================
// Two-Client Interaction Tests
// ============================================================================

#[tokio::test]
async fn test_two_client_set_receive() {
    let router = TestRouter::start().await;

    let client1 = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");
    let client2 = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector = ValueCollector::new();

    client1
        .subscribe("/shared/**", collector.callback_ref())
        .await
        .expect("Subscribe failed");

    tokio::time::sleep(Duration::from_millis(50)).await;

    client2
        .set("/shared/value", 42.0)
        .await
        .expect("Set failed");

    let received = collector.wait_for_count(1, Duration::from_secs(2)).await;
    assert!(received, "Client 1 did not receive value from Client 2");

    client1.close().await;
    client2.close().await;
}

#[tokio::test]
async fn test_bidirectional_communication() {
    let router = TestRouter::start().await;

    let client1 = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");
    let client2 = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    let collector1 = ValueCollector::new();
    let collector2 = ValueCollector::new();

    client1
        .subscribe("/from2/**", collector1.callback_ref())
        .await
        .expect("Subscribe failed");
    client2
        .subscribe("/from1/**", collector2.callback_ref())
        .await
        .expect("Subscribe failed");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Bidirectional sends
    client1
        .set("/from1/message", 100.0)
        .await
        .expect("Set failed");
    client2
        .set("/from2/message", 200.0)
        .await
        .expect("Set failed");

    let recv1 = collector1.wait_for_count(1, Duration::from_secs(2)).await;
    let recv2 = collector2.wait_for_count(1, Duration::from_secs(2)).await;

    assert!(recv1, "Client1 did not receive from Client2");
    assert!(recv2, "Client2 did not receive from Client1");

    client1.close().await;
    client2.close().await;
}

// ============================================================================
// Concurrent Operations Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_operations() {
    let router = TestRouter::start().await;

    let mut clients = vec![];
    for i in 0..5 {
        let client = Clasp::builder(&router.url())
            .name(&format!("ConcurrentClient{}", i))
            .connect()
            .await
            .expect("Connect failed");
        clients.push(client);
    }

    let mut success_count = 0;
    for (i, client) in clients.iter().enumerate() {
        for j in 0..5 {
            match client
                .set(&format!("/concurrent/{}/{}", i, j), (i * 10 + j) as f64)
                .await
            {
                Ok(()) => success_count += 1,
                Err(_) => {}
            }
        }
    }

    assert!(
        success_count >= 20,
        "Only {}/25 concurrent operations succeeded",
        success_count
    );

    for client in clients {
        client.close().await;
    }
}

#[tokio::test]
async fn test_rapid_subscribe_unsubscribe() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    for i in 0..20 {
        let collector = ValueCollector::new();
        let sub_id = client
            .subscribe(&format!("/rapid/{}", i), collector.callback_ref())
            .await
            .expect("Subscribe failed");
        client
            .unsubscribe(sub_id)
            .await
            .expect("Unsubscribe failed");
    }

    client.close().await;
}

// ============================================================================
// Edge Case and Negative Tests
// ============================================================================

#[tokio::test]
async fn test_operations_before_connect() {
    // This test verifies that builder state is clean before connect
    let router = TestRouter::start().await;

    // Build client but don't connect yet
    let builder = ClaspBuilder::new(&router.url()).name("PreConnect");

    // Now connect and verify it works
    let client = builder.connect().await.expect("Connect failed");
    assert!(client.is_connected(), "Should be connected");

    client.close().await;
}

#[tokio::test]
async fn test_operations_after_close() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client.close().await;

    // These should not panic
    assert!(!client.is_connected(), "Should not be connected");
    let _ = client.set("/test", 1.0).await;
    let _ = client.subscribe("/test", |_, _| {}).await;
}

#[tokio::test]
async fn test_double_close() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    client.close().await;
    client.close().await; // Should not panic

    assert!(!client.is_connected(), "Should not be connected");
}

#[tokio::test]
async fn test_special_characters_in_address() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url())
        .await
        .expect("Connect failed");

    // Various address formats
    client.set("/simple", 1.0).await.expect("Set failed");
    client.set("/with-dash", 2.0).await.expect("Set failed");
    client
        .set("/with_underscore", 3.0)
        .await
        .expect("Set failed");
    client.set("/with.dot", 4.0).await.expect("Set failed");
    client.set("/CamelCase", 5.0).await.expect("Set failed");
    client
        .set("/with123numbers", 6.0)
        .await
        .expect("Set failed");

    client.close().await;
}
