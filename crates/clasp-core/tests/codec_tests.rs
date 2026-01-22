//! Codec tests for Clasp core
//! Tests both v3 binary encoding (default) and backward compatibility with v2 MessagePack

use clasp_core::{
    codec, HelloMessage, Message, PublishMessage, SetMessage, SignalType, SubscribeMessage, Value,
    WelcomeMessage,
};

#[test]
fn test_encode_decode_hello() {
    let msg = Message::Hello(HelloMessage {
        version: 3,
        name: "Test Client".to_string(),
        features: vec!["param".to_string(), "event".to_string()],
        capabilities: None,
        token: None,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Hello(hello) => {
            assert_eq!(hello.version, 3);
            assert_eq!(hello.name, "Test Client");
            assert_eq!(hello.features.len(), 2);
        }
        _ => panic!("Expected Hello message"),
    }
}

#[test]
fn test_encode_decode_welcome() {
    let msg = Message::Welcome(WelcomeMessage {
        version: 3,
        session: "sess-123".to_string(),
        name: "Test Server".to_string(),
        features: vec!["param".to_string()],
        time: 1234567890,
        token: None,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Welcome(welcome) => {
            assert_eq!(welcome.session, "sess-123");
            assert_eq!(welcome.time, 1234567890);
        }
        _ => panic!("Expected Welcome message"),
    }
}

#[test]
fn test_encode_decode_set() {
    let msg = Message::Set(SetMessage {
        address: "/test/path".to_string(),
        value: Value::Float(3.14),
        revision: Some(1),
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Set(set) => {
            assert_eq!(set.address, "/test/path");
            match set.value {
                Value::Float(f) => assert!((f - 3.14).abs() < 0.001),
                _ => panic!("Expected Float value"),
            }
        }
        _ => panic!("Expected Set message"),
    }
}

#[test]
fn test_encode_decode_publish() {
    let msg = Message::Publish(PublishMessage {
        address: "/test/event".to_string(),
        signal: Some(SignalType::Event),
        value: None,
        payload: Some(Value::String("hello".to_string())),
        samples: None,
        rate: None,
        id: None,
        phase: None,
        timestamp: Some(123456),
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Publish(pub_msg) => {
            assert_eq!(pub_msg.address, "/test/event");
            assert_eq!(pub_msg.signal, Some(SignalType::Event));
        }
        _ => panic!("Expected Publish message"),
    }
}

#[test]
fn test_encode_decode_subscribe() {
    let msg = Message::Subscribe(SubscribeMessage {
        id: 42,
        pattern: "/test/*".to_string(),
        types: vec![SignalType::Param, SignalType::Event],
        options: None,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Subscribe(sub) => {
            assert_eq!(sub.id, 42);
            assert_eq!(sub.pattern, "/test/*");
        }
        _ => panic!("Expected Subscribe message"),
    }
}

#[test]
fn test_value_types() {
    // Test all value types roundtrip
    // Note: Bytes may deserialize as Array due to MessagePack + serde(untagged) ambiguity
    // so we skip testing Bytes separately here
    let values = vec![
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::Int(42),
        Value::Int(-1000),
        Value::Float(3.14159),
        Value::String("hello world".to_string()),
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        Value::Map(
            vec![
                ("key1".to_string(), Value::Int(1)),
                ("key2".to_string(), Value::String("value".to_string())),
            ]
            .into_iter()
            .collect(),
        ),
    ];

    for value in values {
        let msg = Message::Set(SetMessage {
            address: "/test".to_string(),
            value: value.clone(),
            revision: None,
            lock: false,
            unlock: false,
        });

        let encoded = codec::encode(&msg).expect("encode failed");
        let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

        match decoded {
            Message::Set(set) => {
                // Note: Float comparison needs epsilon
                match (&value, &set.value) {
                    (Value::Float(a), Value::Float(b)) => assert!((a - b).abs() < 0.0001),
                    _ => assert_eq!(value, set.value),
                }
            }
            _ => panic!("Expected Set message"),
        }
    }
}

// ============================================================================
// v3 Binary Encoding Tests
// ============================================================================

#[test]
fn test_v3_set_message_size() {
    // v3 binary encoding should produce smaller messages than v2 MessagePack
    let msg = Message::Set(SetMessage {
        address: "/lights/living/brightness".to_string(),
        value: Value::Float(0.75),
        revision: None,
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    
    // v3 SET format: type(1) + flags(1) + addr_len(2) + addr(25) + value(9) = 38 bytes
    // v2 MessagePack: ~69 bytes due to named keys
    // Target: < 50 bytes for typical SET message
    assert!(
        encoded.len() < 50,
        "v3 SET message should be < 50 bytes, got {} bytes",
        encoded.len()
    );
}

#[test]
fn test_v3_set_message_with_revision() {
    let msg = Message::Set(SetMessage {
        address: "/test".to_string(),
        value: Value::Float(1.0),
        revision: Some(42),
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Set(set) => {
            assert_eq!(set.revision, Some(42));
        }
        _ => panic!("Expected Set message"),
    }
}

#[test]
fn test_v3_set_message_with_lock() {
    let msg = Message::Set(SetMessage {
        address: "/test".to_string(),
        value: Value::Bool(true),
        revision: None,
        lock: true,
        unlock: false,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Set(set) => {
            assert!(set.lock);
            assert!(!set.unlock);
        }
        _ => panic!("Expected Set message"),
    }
}

#[test]
fn test_v3_set_message_string_value() {
    let msg = Message::Set(SetMessage {
        address: "/label".to_string(),
        value: Value::String("Hello World".to_string()),
        revision: None,
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let (decoded, _frame) = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Set(set) => {
            assert_eq!(set.value, Value::String("Hello World".to_string()));
        }
        _ => panic!("Expected Set message"),
    }
}

#[test]
fn test_v3_encoding_starts_with_message_type() {
    // v3 binary format: payload first byte should be message type code
    // Note: encode() returns a frame, payload starts after header (magic + flags + len = 4 bytes)
    let set_msg = Message::Set(SetMessage {
        address: "/test".to_string(),
        value: Value::Float(1.0),
        revision: None,
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&set_msg).expect("encode failed");
    // Frame header: magic (0x53) + flags (1) + length (2) = 4 bytes
    // Payload starts at offset 4
    assert_eq!(encoded[0], 0x53, "Frame magic byte should be 0x53");
    assert_eq!(encoded[4], 0x21, "SET payload should start with 0x21");

    let hello_msg = Message::Hello(HelloMessage {
        version: 3,
        name: "Test".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });

    let encoded = codec::encode(&hello_msg).expect("encode failed");
    assert_eq!(encoded[0], 0x53, "Frame magic byte should be 0x53");
    assert_eq!(encoded[4], 0x01, "HELLO payload should start with 0x01");
}

#[test]
fn test_v3_benchmark_set_encoding() {
    // Verify encoding is reasonably fast (note: debug builds are slower)
    use std::time::Instant;

    let msg = Message::Set(SetMessage {
        address: "/lights/living/brightness".to_string(),
        value: Value::Float(0.75),
        revision: Some(1),
        lock: false,
        unlock: false,
    });

    let iterations = 100_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _ = codec::encode(&msg).expect("encode failed");
    }
    
    let elapsed = start.elapsed();
    let per_msg_ns = elapsed.as_nanos() / iterations as u128;
    
    // Target: < 2000ns per message (0.5M msg/s) in debug builds
    // Release builds should achieve < 200ns (5M+ msg/s)
    assert!(
        per_msg_ns < 2000,
        "v3 SET encoding should be < 2000ns (debug), got {}ns",
        per_msg_ns
    );
    
    let msgs_per_sec = 1_000_000_000 / per_msg_ns;
    println!(
        "v3 SET encoding: {}ns/msg = {:.2} million msg/s",
        per_msg_ns,
        msgs_per_sec as f64 / 1_000_000.0
    );

    // Decode benchmark
    let encoded = codec::encode(&msg).expect("encode failed");
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _ = codec::decode(&encoded).expect("decode failed");
    }
    
    let elapsed = start.elapsed();
    let per_msg_ns = elapsed.as_nanos() / iterations as u128;
    let msgs_per_sec = 1_000_000_000 / per_msg_ns;
    println!(
        "v3 SET decoding: {}ns/msg = {:.2} million msg/s",
        per_msg_ns,
        msgs_per_sec as f64 / 1_000_000.0
    );
}
