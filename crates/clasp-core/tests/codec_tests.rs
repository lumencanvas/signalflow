//! Codec tests for Clasp core

use clasp_core::{codec, Message, Value, SetMessage, PublishMessage, HelloMessage, WelcomeMessage, SubscribeMessage, SignalType, QoS};

#[test]
fn test_encode_decode_hello() {
    let msg = Message::Hello(HelloMessage {
        version: 2,
        name: "Test Client".to_string(),
        features: vec!["param".to_string(), "event".to_string()],
        capabilities: None,
        token: None,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let decoded: Message = codec::decode(&encoded).expect("decode failed");

    match decoded {
        Message::Hello(hello) => {
            assert_eq!(hello.version, 2);
            assert_eq!(hello.name, "Test Client");
            assert_eq!(hello.features.len(), 2);
        }
        _ => panic!("Expected Hello message"),
    }
}

#[test]
fn test_encode_decode_welcome() {
    let msg = Message::Welcome(WelcomeMessage {
        version: 2,
        session: "sess-123".to_string(),
        name: "Test Server".to_string(),
        features: vec!["param".to_string()],
        time: 1234567890,
        token: None,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let decoded: Message = codec::decode(&encoded).expect("decode failed");

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
    let decoded: Message = codec::decode(&encoded).expect("decode failed");

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
        timestamp: Some(123456),
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let decoded: Message = codec::decode(&encoded).expect("decode failed");

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
        types: Some(vec![SignalType::Param, SignalType::Event]),
        options: None,
    });

    let encoded = codec::encode(&msg).expect("encode failed");
    let decoded: Message = codec::decode(&encoded).expect("decode failed");

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
    let values = vec![
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::Int(42),
        Value::Int(-1000),
        Value::Float(3.14159),
        Value::String("hello world".to_string()),
        Value::Bytes(vec![0x01, 0x02, 0x03]),
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        Value::Map(vec![
            ("key1".to_string(), Value::Int(1)),
            ("key2".to_string(), Value::String("value".to_string())),
        ].into_iter().collect()),
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
        let decoded: Message = codec::decode(&encoded).expect("decode failed");

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
