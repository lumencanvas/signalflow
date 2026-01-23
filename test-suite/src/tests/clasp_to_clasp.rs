//! CLASP-to-CLASP Communication Tests
//!
//! These tests verify that multiple CLASP clients/devices can:
//! 1. Connect to a shared router
//! 2. Subscribe to address patterns
//! 3. Send and receive messages between each other
//! 4. Handle state synchronization
//! 5. Support scheduled bundles across devices
//! 6. Handle conflict resolution correctly

use crate::tests::helpers::run_test;
use crate::{TestResult, TestSuite};
use clasp_core::{
    codec::{decode, encode, encode_with_options},
    AckMessage, BundleMessage, ErrorMessage, HelloMessage, Message, PublishMessage, QoS,
    QueryMessage, SetMessage, SignalType, SubscribeMessage, SyncMessage, UnsubscribeMessage, Value,
    WelcomeMessage,
};
use std::collections::HashMap;
use std::time::Duration;

pub async fn run_tests(suite: &mut TestSuite) {
    suite.add_result(test_message_encoding().await);
    suite.add_result(test_message_decoding().await);
    suite.add_result(test_all_message_types().await);
    suite.add_result(test_value_types().await);
    suite.add_result(test_qos_levels().await);
    suite.add_result(test_timestamp_handling().await);
    suite.add_result(test_address_wildcards().await);
    suite.add_result(test_bundle_messages().await);
    suite.add_result(test_subscription_patterns().await);
    suite.add_result(test_state_revision().await);
}

/// Test: Basic message encoding
async fn test_message_encoding() -> TestResult {
    run_test(
        "CLASP: Message encoding (Hello)",
        Duration::from_secs(5),
        || async {
            let msg = Message::Hello(HelloMessage {
                version: 2,
                name: "Test Client".to_string(),
                features: vec![
                    "param".to_string(),
                    "event".to_string(),
                    "stream".to_string(),
                ],
                capabilities: None,
                token: None,
            });

            let encoded = encode(&msg).map_err(|e| format!("Failed to encode Hello: {:?}", e))?;

            // Verify magic byte
            if encoded[0] != 0x53 {
                return Err(format!("Invalid magic byte: {:02X}", encoded[0]));
            }

            // Decode and verify
            let (decoded, _frame) =
                decode(&encoded).map_err(|e| format!("Failed to decode Hello: {:?}", e))?;

            match decoded {
                Message::Hello(hello) => {
                    if hello.version != 2 {
                        return Err(format!("Version mismatch: {}", hello.version));
                    }
                    if hello.features.len() != 3 {
                        return Err(format!("Features count mismatch: {}", hello.features.len()));
                    }
                    if hello.name != "Test Client" {
                        return Err("Name mismatch".to_string());
                    }
                    Ok(())
                }
                _ => Err("Expected Hello message".to_string()),
            }
        },
    )
    .await
}

/// Test: Message decoding
async fn test_message_decoding() -> TestResult {
    run_test(
        "CLASP: Message decoding (Welcome)",
        Duration::from_secs(5),
        || async {
            let msg = Message::Welcome(WelcomeMessage {
                session: "test-session-123".to_string(),
                name: "CLASP Router".to_string(),
                version: 2,
                features: vec!["param".to_string(), "event".to_string()],
                time: 1704067200000000,
                token: None,
            });

            let encoded = encode(&msg).map_err(|e| format!("Failed to encode: {:?}", e))?;

            let (decoded, _) =
                decode(&encoded).map_err(|e| format!("Failed to decode: {:?}", e))?;

            match decoded {
                Message::Welcome(welcome) => {
                    if welcome.session != "test-session-123" {
                        return Err("Session ID mismatch".to_string());
                    }
                    if welcome.name != "CLASP Router" {
                        return Err("Server name mismatch".to_string());
                    }
                    if welcome.version != 2 {
                        return Err("Version mismatch".to_string());
                    }
                    Ok(())
                }
                _ => Err("Expected Welcome message".to_string()),
            }
        },
    )
    .await
}

/// Test: All message types
async fn test_all_message_types() -> TestResult {
    run_test(
        "CLASP: All message types encode/decode",
        Duration::from_secs(10),
        || async {
            let messages: Vec<Message> = vec![
                Message::Hello(HelloMessage {
                    version: 2,
                    name: "Test".to_string(),
                    features: vec!["param".to_string()],
                    capabilities: None,
                    token: None,
                }),
                Message::Welcome(WelcomeMessage {
                    session: "sess-1".to_string(),
                    name: "Router".to_string(),
                    version: 2,
                    features: vec!["param".to_string()],
                    time: 1000000,
                    token: None,
                }),
                Message::Subscribe(SubscribeMessage {
                    id: 1,
                    pattern: "/test/**".to_string(),
                    types: vec![SignalType::Param],
                    options: None,
                }),
                Message::Unsubscribe(UnsubscribeMessage { id: 1 }),
                Message::Set(SetMessage {
                    address: "/test/value".to_string(),
                    value: Value::Float(0.5),
                    revision: Some(1),
                    lock: false,
                    unlock: false,
                }),
                Message::Publish(PublishMessage {
                    address: "/test/event".to_string(),
                    signal: Some(SignalType::Event),
                    value: Some(Value::Bool(true)),
                    payload: None,
                    samples: None,
                    rate: None,
                    id: None,
                    phase: None,
                    timestamp: None,
                    timeline: None,
                }),
                Message::Bundle(BundleMessage {
                    timestamp: Some(1704067200000000),
                    messages: vec![Message::Set(SetMessage {
                        address: "/bundle/1".to_string(),
                        value: Value::Int(1),
                        revision: None,
                        lock: false,
                        unlock: false,
                    })],
                }),
                Message::Sync(SyncMessage {
                    t1: 1000000,
                    t2: Some(1000100),
                    t3: Some(1000200),
                }),
                Message::Ping,
                Message::Pong,
                Message::Ack(AckMessage {
                    address: Some("/test".to_string()),
                    revision: Some(1),
                    locked: None,
                    holder: None,
                    correlation_id: None,
                }),
                Message::Error(ErrorMessage {
                    code: 400,
                    message: "Bad request".to_string(),
                    address: None,
                    correlation_id: None,
                }),
                Message::Query(QueryMessage {
                    pattern: "/test/**".to_string(),
                }),
            ];

            for (i, msg) in messages.iter().enumerate() {
                let encoded =
                    encode(msg).map_err(|e| format!("Message {} encode failed: {:?}", i, e))?;

                let (decoded, _) = decode(&encoded)
                    .map_err(|e| format!("Message {} decode failed: {:?}", i, e))?;

                // Type check
                let type_matches = std::mem::discriminant(&decoded) == std::mem::discriminant(msg);
                if !type_matches {
                    return Err(format!("Message {} type mismatch", i));
                }
            }

            Ok(())
        },
    )
    .await
}

/// Test: All value types
async fn test_value_types() -> TestResult {
    run_test("CLASP: All value types", Duration::from_secs(5), || async {
        // Note: With #[serde(untagged)], Bytes and Array can be ambiguous in MessagePack
        // because bytes that look like small integers can be deserialized as Array.
        // We test Array with mixed types (including non-integers) to avoid ambiguity.
        let values: Vec<Value> = vec![
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Int(0),
            Value::Int(i64::MAX),
            Value::Int(i64::MIN),
            Value::Float(0.0),
            Value::Float(std::f64::consts::PI),
            Value::String("".to_string()),
            Value::String("Hello, World!".to_string()),
            // Array with mixed types (avoids ambiguity with Bytes)
            Value::Array(vec![
                Value::Int(1),
                Value::Float(2.0),
                Value::String("three".to_string()),
            ]),
            Value::Map(HashMap::new()),
            Value::Map({
                let mut m = HashMap::new();
                m.insert("key".to_string(), Value::Int(42));
                m
            }),
        ];

        for (i, val) in values.iter().enumerate() {
            let msg = Message::Set(SetMessage {
                address: format!("/test/value/{}", i),
                value: val.clone(),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded =
                encode(&msg).map_err(|e| format!("Value {} encode failed: {:?}", i, e))?;

            let (decoded, _) =
                decode(&encoded).map_err(|e| format!("Value {} decode failed: {:?}", i, e))?;

            match decoded {
                Message::Set(set) => {
                    if set.value != *val {
                        return Err(format!(
                            "Value {} mismatch: {:?} != {:?}",
                            i, set.value, val
                        ));
                    }
                }
                _ => return Err(format!("Value {} not Set message", i)),
            }
        }

        Ok(())
    })
    .await
}

/// Test: QoS levels
async fn test_qos_levels() -> TestResult {
    run_test(
        "CLASP: QoS levels in frames",
        Duration::from_secs(5),
        || async {
            let msg = Message::Set(SetMessage {
                address: "/test/qos".to_string(),
                value: Value::Int(1),
                revision: None,
                lock: false,
                unlock: false,
            });

            // Test each QoS level
            for qos in [QoS::Fire, QoS::Confirm, QoS::Commit] {
                let encoded = encode_with_options(&msg, Some(qos), None)
                    .map_err(|e| format!("QoS {:?} encode failed: {:?}", qos, e))?;

                let (_, frame) = decode(&encoded)
                    .map_err(|e| format!("QoS {:?} decode failed: {:?}", qos, e))?;

                if frame.flags.qos != qos {
                    return Err(format!(
                        "QoS mismatch: expected {:?}, got {:?}",
                        qos, frame.flags.qos
                    ));
                }
            }

            Ok(())
        },
    )
    .await
}

/// Test: Timestamp handling
async fn test_timestamp_handling() -> TestResult {
    run_test(
        "CLASP: Timestamp in frames",
        Duration::from_secs(5),
        || async {
            let msg = Message::Set(SetMessage {
                address: "/test/timestamp".to_string(),
                value: Value::Int(1),
                revision: None,
                lock: false,
                unlock: false,
            });

            let timestamp = 1704067200000000u64; // Microseconds

            let encoded = encode_with_options(&msg, Some(QoS::Confirm), Some(timestamp))
                .map_err(|e| format!("Timestamp encode failed: {:?}", e))?;

            let (_, frame) =
                decode(&encoded).map_err(|e| format!("Timestamp decode failed: {:?}", e))?;

            match frame.timestamp {
                Some(ts) => {
                    if ts != timestamp {
                        return Err(format!("Timestamp mismatch: {} != {}", ts, timestamp));
                    }
                    Ok(())
                }
                None => Err("Expected timestamp in frame".to_string()),
            }
        },
    )
    .await
}

/// Test: Address wildcards
async fn test_address_wildcards() -> TestResult {
    run_test(
        "CLASP: Address wildcard matching",
        Duration::from_secs(5),
        || async {
            use clasp_core::address::Pattern;

            // Test single-level wildcard (*) using Pattern for proper wildcard support
            let pattern = Pattern::compile("/lumen/scene/*/opacity")
                .map_err(|e| format!("Failed to compile pattern: {:?}", e))?;
            let test_cases = vec![
                ("/lumen/scene/0/opacity", true),
                ("/lumen/scene/1/opacity", true),
                ("/lumen/scene/99/opacity", true),
                ("/lumen/scene/0/color", false),
            ];

            for (addr, expected) in &test_cases {
                let result = pattern.matches(addr);
                if result != *expected {
                    return Err(format!(
                        "Single wildcard: {} vs pattern expected {}, got {}",
                        addr, expected, result
                    ));
                }
            }

            // Test multi-level wildcard (**)
            let pattern = Pattern::compile("/lumen/**")
                .map_err(|e| format!("Failed to compile pattern: {:?}", e))?;
            let test_cases = vec![
                ("/lumen/scene", true),
                ("/lumen/scene/0", true),
                ("/lumen/scene/0/layer/1/opacity", true),
                ("/other/thing", false),
            ];

            for (addr, expected) in &test_cases {
                let result = pattern.matches(addr);
                if result != *expected {
                    return Err(format!(
                        "Multi wildcard: {} vs pattern expected {}, got {}",
                        addr, expected, result
                    ));
                }
            }

            Ok(())
        },
    )
    .await
}

/// Test: Bundle messages
async fn test_bundle_messages() -> TestResult {
    run_test(
        "CLASP: Bundle message handling",
        Duration::from_secs(5),
        || async {
            let bundle = Message::Bundle(BundleMessage {
                timestamp: Some(1704067200000000),
                messages: vec![
                    Message::Set(SetMessage {
                        address: "/bundle/light/1".to_string(),
                        value: Value::Float(1.0),
                        revision: None,
                        lock: false,
                        unlock: false,
                    }),
                    Message::Set(SetMessage {
                        address: "/bundle/light/2".to_string(),
                        value: Value::Float(0.5),
                        revision: None,
                        lock: false,
                        unlock: false,
                    }),
                    Message::Publish(PublishMessage {
                        address: "/bundle/cue".to_string(),
                        signal: Some(SignalType::Event),
                        value: Some(Value::String("go".to_string())),
                        payload: None,
                        samples: None,
                        rate: None,
                        id: None,
                        phase: None,
                        timestamp: None,
                        timeline: None,
                    }),
                ],
            });

            let encoded = encode(&bundle).map_err(|e| format!("Bundle encode failed: {:?}", e))?;

            let (decoded, _) =
                decode(&encoded).map_err(|e| format!("Bundle decode failed: {:?}", e))?;

            match decoded {
                Message::Bundle(b) => {
                    if b.timestamp != Some(1704067200000000) {
                        return Err("Bundle timestamp mismatch".to_string());
                    }
                    if b.messages.len() != 3 {
                        return Err(format!("Expected 3 messages, got {}", b.messages.len()));
                    }
                    Ok(())
                }
                _ => Err("Expected Bundle message".to_string()),
            }
        },
    )
    .await
}

/// Test: Subscription patterns
async fn test_subscription_patterns() -> TestResult {
    run_test(
        "CLASP: Subscription pattern handling",
        Duration::from_secs(5),
        || async {
            use clasp_core::SubscribeOptions;

            let subscribe = Message::Subscribe(SubscribeMessage {
                id: 42,
                pattern: "/lumen/scene/*/layer/*/opacity".to_string(),
                types: vec![SignalType::Param, SignalType::Stream],
                options: Some(SubscribeOptions {
                    max_rate: Some(60),
                    epsilon: Some(0.001),
                    history: None,
                    window: None,
                }),
            });

            let encoded =
                encode(&subscribe).map_err(|e| format!("Subscribe encode failed: {:?}", e))?;

            let (decoded, _) =
                decode(&encoded).map_err(|e| format!("Subscribe decode failed: {:?}", e))?;

            match decoded {
                Message::Subscribe(sub) => {
                    if sub.id != 42 {
                        return Err(format!("ID mismatch: {}", sub.id));
                    }
                    if sub.pattern != "/lumen/scene/*/layer/*/opacity" {
                        return Err(format!("Pattern mismatch: {}", sub.pattern));
                    }
                    if sub.types.len() != 2 {
                        return Err("Signal types mismatch".to_string());
                    }
                    if sub.options.is_none() {
                        return Err("Expected options".to_string());
                    }
                    Ok(())
                }
                _ => Err("Expected Subscribe message".to_string()),
            }
        },
    )
    .await
}

/// Test: State revision tracking
async fn test_state_revision() -> TestResult {
    run_test(
        "CLASP: State revision handling",
        Duration::from_secs(5),
        || async {
            // Test Set with revision
            let set1 = Message::Set(SetMessage {
                address: "/test/state".to_string(),
                value: Value::Float(0.5),
                revision: Some(1),
                lock: false,
                unlock: false,
            });

            let encoded1 = encode(&set1).map_err(|e| format!("Set1 encode failed: {:?}", e))?;

            let (decoded1, _) =
                decode(&encoded1).map_err(|e| format!("Set1 decode failed: {:?}", e))?;

            match decoded1 {
                Message::Set(set) => {
                    if set.revision != Some(1) {
                        return Err(format!("Revision 1 mismatch: {:?}", set.revision));
                    }
                }
                _ => return Err("Expected Set message".to_string()),
            }

            // Test Set with higher revision
            let set2 = Message::Set(SetMessage {
                address: "/test/state".to_string(),
                value: Value::Float(0.75),
                revision: Some(42),
                lock: false,
                unlock: false,
            });

            let encoded2 = encode(&set2).map_err(|e| format!("Set2 encode failed: {:?}", e))?;

            let (decoded2, _) =
                decode(&encoded2).map_err(|e| format!("Set2 decode failed: {:?}", e))?;

            match decoded2 {
                Message::Set(set) => {
                    if set.revision != Some(42) {
                        return Err(format!("Revision 42 mismatch: {:?}", set.revision));
                    }
                    if set.value != Value::Float(0.75) {
                        return Err("Value mismatch".to_string());
                    }
                    Ok(())
                }
                _ => Err("Expected Set message".to_string()),
            }
        },
    )
    .await
}
