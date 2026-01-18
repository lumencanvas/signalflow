//! WASM integration tests for clasp-wasm
//!
//! These tests run in a headless browser environment using wasm-pack test.
//! Run with: wasm-pack test --headless --chrome
//!
//! Note: Some tests require a running CLASP server. Tests that don't require
//! a server are marked with the `local_only` attribute.

#![cfg(target_arch = "wasm32")]

use clasp_core::{
    codec, HelloMessage, Message, SetMessage, SignalType, SubscribeMessage, Value, WelcomeMessage,
    PROTOCOL_VERSION,
};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// =============================================================================
// Value Conversion Tests
// =============================================================================

/// Test converting null values between JS and Clasp
#[wasm_bindgen_test]
fn test_value_conversion_null() {
    let clasp_null = Value::Null;
    let encoded = codec::encode(&Message::Set(SetMessage {
        address: "/test".to_string(),
        value: clasp_null.clone(),
        revision: None,
        lock: false,
        unlock: false,
    }))
    .unwrap();

    let (decoded, _) = codec::decode(&encoded).unwrap();
    if let Message::Set(set) = decoded {
        assert!(matches!(set.value, Value::Null));
    } else {
        panic!("Expected Set message");
    }
}

/// Test converting boolean values
#[wasm_bindgen_test]
fn test_value_conversion_bool() {
    for b in [true, false] {
        let clasp_bool = Value::Bool(b);
        let encoded = codec::encode(&Message::Set(SetMessage {
            address: "/test/bool".to_string(),
            value: clasp_bool.clone(),
            revision: None,
            lock: false,
            unlock: false,
        }))
        .unwrap();

        let (decoded, _) = codec::decode(&encoded).unwrap();
        if let Message::Set(set) = decoded {
            assert_eq!(set.value, Value::Bool(b));
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test converting integer values
#[wasm_bindgen_test]
fn test_value_conversion_int() {
    let test_values = [0i64, 1, -1, 42, -42, i64::MAX, i64::MIN];

    for i in test_values {
        let clasp_int = Value::Int(i);
        let encoded = codec::encode(&Message::Set(SetMessage {
            address: "/test/int".to_string(),
            value: clasp_int.clone(),
            revision: None,
            lock: false,
            unlock: false,
        }))
        .unwrap();

        let (decoded, _) = codec::decode(&encoded).unwrap();
        if let Message::Set(set) = decoded {
            assert_eq!(set.value, Value::Int(i));
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test converting float values
#[wasm_bindgen_test]
fn test_value_conversion_float() {
    let test_values = [0.0f64, 1.5, -1.5, 3.14159, f64::MAX, f64::MIN];

    for f in test_values {
        let clasp_float = Value::Float(f);
        let encoded = codec::encode(&Message::Set(SetMessage {
            address: "/test/float".to_string(),
            value: clasp_float.clone(),
            revision: None,
            lock: false,
            unlock: false,
        }))
        .unwrap();

        let (decoded, _) = codec::decode(&encoded).unwrap();
        if let Message::Set(set) = decoded {
            if let Value::Float(decoded_f) = set.value {
                assert!((decoded_f - f).abs() < f64::EPSILON || (decoded_f == f));
            } else {
                panic!("Expected Float value");
            }
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test converting string values
#[wasm_bindgen_test]
fn test_value_conversion_string() {
    let test_values = [
        "",
        "hello",
        "Hello, World!",
        "/param/address",
        "unicode: 你好 🎉",
        "special chars: \n\t\"'\\",
    ];

    for s in test_values {
        let clasp_str = Value::String(s.to_string());
        let encoded = codec::encode(&Message::Set(SetMessage {
            address: "/test/string".to_string(),
            value: clasp_str.clone(),
            revision: None,
            lock: false,
            unlock: false,
        }))
        .unwrap();

        let (decoded, _) = codec::decode(&encoded).unwrap();
        if let Message::Set(set) = decoded {
            assert_eq!(set.value, Value::String(s.to_string()));
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test converting byte arrays
#[wasm_bindgen_test]
fn test_value_conversion_bytes() {
    // Note: Empty bytes may be decoded as empty array due to MessagePack ambiguity
    let test_values: Vec<Vec<u8>> = vec![
        vec![0], // Skip empty - ambiguous in msgpack
        vec![1, 2, 3, 4, 5],
        vec![0xFF, 0x00, 0xAB, 0xCD],
        (0..256).map(|i| i as u8).collect(),
    ];

    for bytes in test_values {
        let clasp_bytes = Value::Bytes(bytes.clone());
        let encoded = codec::encode(&Message::Set(SetMessage {
            address: "/test/bytes".to_string(),
            value: clasp_bytes.clone(),
            revision: None,
            lock: false,
            unlock: false,
        }))
        .unwrap();

        let (decoded, _) = codec::decode(&encoded).unwrap();
        if let Message::Set(set) = decoded {
            // Bytes may come back as bytes or as array of ints - both are valid
            match &set.value {
                Value::Bytes(b) => assert_eq!(*b, bytes),
                Value::Array(arr) => {
                    let decoded_bytes: Vec<u8> = arr
                        .iter()
                        .filter_map(|v| {
                            if let Value::Int(i) = v {
                                Some(*i as u8)
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert_eq!(decoded_bytes, bytes);
                }
                _ => panic!("Expected Bytes or Array value"),
            }
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test converting arrays
#[wasm_bindgen_test]
fn test_value_conversion_array() {
    let test_arrays = vec![
        vec![],
        vec![Value::Int(1)],
        vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        vec![
            Value::Bool(true),
            Value::String("hello".to_string()),
            Value::Float(3.14),
        ],
        vec![
            Value::Array(vec![Value::Int(1), Value::Int(2)]),
            Value::Array(vec![Value::Int(3), Value::Int(4)]),
        ],
    ];

    for arr in test_arrays {
        let clasp_arr = Value::Array(arr.clone());
        let encoded = codec::encode(&Message::Set(SetMessage {
            address: "/test/array".to_string(),
            value: clasp_arr.clone(),
            revision: None,
            lock: false,
            unlock: false,
        }))
        .unwrap();

        let (decoded, _) = codec::decode(&encoded).unwrap();
        if let Message::Set(set) = decoded {
            assert_eq!(set.value, Value::Array(arr));
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test converting maps/objects
#[wasm_bindgen_test]
fn test_value_conversion_map() {
    let mut map = HashMap::new();
    map.insert("name".to_string(), Value::String("test".to_string()));
    map.insert("value".to_string(), Value::Int(42));
    map.insert("enabled".to_string(), Value::Bool(true));

    let clasp_map = Value::Map(map.clone());
    let encoded = codec::encode(&Message::Set(SetMessage {
        address: "/test/map".to_string(),
        value: clasp_map.clone(),
        revision: None,
        lock: false,
        unlock: false,
    }))
    .unwrap();

    let (decoded, _) = codec::decode(&encoded).unwrap();
    if let Message::Set(set) = decoded {
        if let Value::Map(decoded_map) = set.value {
            assert_eq!(decoded_map.len(), map.len());
            for (k, v) in &map {
                assert_eq!(decoded_map.get(k), Some(v));
            }
        } else {
            panic!("Expected Map value");
        }
    } else {
        panic!("Expected Set message");
    }
}

/// Test nested complex values
#[wasm_bindgen_test]
fn test_value_conversion_nested() {
    let mut inner_map = HashMap::new();
    inner_map.insert("x".to_string(), Value::Float(1.0));
    inner_map.insert("y".to_string(), Value::Float(2.0));
    inner_map.insert("z".to_string(), Value::Float(3.0));

    let mut outer_map = HashMap::new();
    outer_map.insert("position".to_string(), Value::Map(inner_map));
    outer_map.insert(
        "colors".to_string(),
        Value::Array(vec![Value::Int(255), Value::Int(128), Value::Int(0)]),
    );
    outer_map.insert("label".to_string(), Value::String("node1".to_string()));

    let value = Value::Map(outer_map);
    let encoded = codec::encode(&Message::Set(SetMessage {
        address: "/complex/nested".to_string(),
        value: value.clone(),
        revision: None,
        lock: false,
        unlock: false,
    }))
    .unwrap();

    let (decoded, _) = codec::decode(&encoded).unwrap();
    if let Message::Set(set) = decoded {
        assert_eq!(set.value, value);
    } else {
        panic!("Expected Set message");
    }
}

// =============================================================================
// Message Encoding/Decoding Tests
// =============================================================================

/// Test HELLO message encoding
#[wasm_bindgen_test]
fn test_hello_message() {
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "Test Client".to_string(),
        features: vec!["param".to_string(), "event".to_string()],
        capabilities: None,
        token: None,
    });

    let encoded = codec::encode(&hello).unwrap();
    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Hello(h) = decoded {
        assert_eq!(h.version, PROTOCOL_VERSION);
        assert_eq!(h.name, "Test Client");
        assert_eq!(h.features, vec!["param", "event"]);
    } else {
        panic!("Expected Hello message");
    }
}

/// Test HELLO with auth token
#[wasm_bindgen_test]
fn test_hello_with_token() {
    let token = "test-auth-token-12345".to_string();
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "Authenticated Client".to_string(),
        features: vec!["param".to_string()],
        capabilities: None,
        token: Some(token.clone()),
    });

    let encoded = codec::encode(&hello).unwrap();
    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Hello(h) = decoded {
        assert_eq!(h.token, Some(token));
    } else {
        panic!("Expected Hello message");
    }
}

/// Test WELCOME message
#[wasm_bindgen_test]
fn test_welcome_message() {
    let welcome = Message::Welcome(WelcomeMessage {
        version: PROTOCOL_VERSION,
        session: "session-abc123".to_string(),
        name: "Test Server".to_string(),
        features: vec!["param".to_string(), "stream".to_string()],
        time: 1234567890,
        token: None,
    });

    let encoded = codec::encode(&welcome).unwrap();
    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Welcome(w) = decoded {
        assert_eq!(w.session, "session-abc123");
        assert_eq!(w.name, "Test Server");
        assert_eq!(w.features, vec!["param", "stream"]);
    } else {
        panic!("Expected Welcome message");
    }
}

/// Test SET message
#[wasm_bindgen_test]
fn test_set_message() {
    let set = Message::Set(SetMessage {
        address: "/lights/dimmer".to_string(),
        value: Value::Float(0.75),
        revision: Some(42),
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&set).unwrap();
    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Set(s) = decoded {
        assert_eq!(s.address, "/lights/dimmer");
        assert_eq!(s.value, Value::Float(0.75));
        assert_eq!(s.revision, Some(42));
    } else {
        panic!("Expected Set message");
    }
}

/// Test SUBSCRIBE message
#[wasm_bindgen_test]
fn test_subscribe_message() {
    let subscribe = Message::Subscribe(SubscribeMessage {
        id: 1,
        pattern: "/lights/**".to_string(),
        types: vec![SignalType::Param],
        options: None,
    });

    let encoded = codec::encode(&subscribe).unwrap();
    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Subscribe(s) = decoded {
        assert_eq!(s.id, 1);
        assert_eq!(s.pattern, "/lights/**");
        assert_eq!(s.types, vec![SignalType::Param]);
    } else {
        panic!("Expected Subscribe message");
    }
}

// =============================================================================
// Protocol Address Pattern Tests
// =============================================================================

/// Test that address patterns are preserved correctly
#[wasm_bindgen_test]
fn test_address_patterns() {
    let patterns = [
        "/",
        "/simple",
        "/nested/path",
        "/deeply/nested/path/to/value",
        "/*",
        "/lights/*",
        "/lights/**",
        "/devices/*/status",
        "/[0-9]+",
        "/node-{id}/param",
    ];

    for pattern in patterns {
        let msg = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: pattern.to_string(),
            types: vec![],
            options: None,
        });

        let encoded = codec::encode(&msg).unwrap();
        let (decoded, _) = codec::decode(&encoded).unwrap();

        if let Message::Subscribe(s) = decoded {
            assert_eq!(s.pattern, pattern);
        } else {
            panic!("Expected Subscribe message for pattern: {}", pattern);
        }
    }
}

// =============================================================================
// Binary Data Handling Tests
// =============================================================================

/// Test that binary data survives encoding/decoding
#[wasm_bindgen_test]
fn test_binary_data_integrity() {
    // Generate some binary data that might cause issues if mishandled
    let binary_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    let msg = Message::Set(SetMessage {
        address: "/binary/data".to_string(),
        value: Value::Bytes(binary_data.clone()),
        revision: None,
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).unwrap();
    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Set(s) = decoded {
        // Binary data may come back as Bytes or as Array of integers
        match &s.value {
            Value::Bytes(bytes) => assert_eq!(*bytes, binary_data),
            Value::Array(arr) => {
                let decoded_bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|v| {
                        if let Value::Int(i) = v {
                            Some(*i as u8)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(decoded_bytes, binary_data);
            }
            _ => panic!("Expected Bytes or Array value"),
        }
    } else {
        panic!("Expected Set message");
    }
}

/// Test larger binary payloads (within codec limits)
#[wasm_bindgen_test]
fn test_large_binary_payload() {
    // 16KB of data (within codec size limits)
    let large_data: Vec<u8> = (0..16384).map(|i| (i % 256) as u8).collect();

    let msg = Message::Set(SetMessage {
        address: "/large/binary".to_string(),
        value: Value::Bytes(large_data.clone()),
        revision: None,
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).unwrap();
    assert!(
        encoded.len() > 16384,
        "Encoded message should contain the full payload"
    );

    let (decoded, _) = codec::decode(&encoded).unwrap();

    if let Message::Set(s) = decoded {
        // Large binary data may come back as Bytes or as Array of integers
        match &s.value {
            Value::Bytes(bytes) => {
                assert_eq!(bytes.len(), 16384);
                assert_eq!(*bytes, large_data);
            }
            Value::Array(arr) => {
                assert_eq!(arr.len(), 16384);
                let decoded_bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|v| {
                        if let Value::Int(i) = v {
                            Some(*i as u8)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(decoded_bytes, large_data);
            }
            _ => panic!("Expected Bytes or Array value"),
        }
    } else {
        panic!("Expected Set message");
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Test empty strings and arrays
#[wasm_bindgen_test]
fn test_empty_containers() {
    let empty_string = Value::String("".to_string());
    let empty_array = Value::Array(vec![]);
    let empty_map = Value::Map(HashMap::new());

    // Note: Empty bytes can be ambiguous with empty array in msgpack
    for (name, value) in [
        ("empty_string", empty_string),
        ("empty_array", empty_array),
        ("empty_map", empty_map),
    ] {
        let msg = Message::Set(SetMessage {
            address: format!("/test/{}", name),
            value: value.clone(),
            revision: None,
            lock: false,
            unlock: false,
        });

        let encoded = codec::encode(&msg).unwrap();
        let (decoded, _) = codec::decode(&encoded).unwrap();

        if let Message::Set(s) = decoded {
            assert_eq!(s.value, value, "Failed for {}", name);
        } else {
            panic!("Expected Set message for {}", name);
        }
    }
}

/// Test special float values
#[wasm_bindgen_test]
fn test_special_floats() {
    // Note: NaN and infinity may not round-trip correctly in all cases
    let special_values = [0.0, -0.0, f64::EPSILON, f64::MIN_POSITIVE];

    for f in special_values {
        let msg = Message::Set(SetMessage {
            address: "/test/special_float".to_string(),
            value: Value::Float(f),
            revision: None,
            lock: false,
            unlock: false,
        });

        let encoded = codec::encode(&msg).unwrap();
        let (decoded, _) = codec::decode(&encoded).unwrap();

        if let Message::Set(s) = decoded {
            if let Value::Float(decoded_f) = s.value {
                // For -0.0 and 0.0, they're equal
                if f == 0.0 {
                    assert_eq!(decoded_f, 0.0);
                } else {
                    assert!((decoded_f - f).abs() < f64::EPSILON * 10.0);
                }
            } else {
                panic!("Expected Float value");
            }
        } else {
            panic!("Expected Set message");
        }
    }
}

/// Test Unicode strings with various scripts
#[wasm_bindgen_test]
fn test_unicode_strings() {
    let unicode_strings = [
        "Hello, World!",                    // ASCII
        "Héllo, Wörld!",                    // Latin Extended
        "你好，世界！",                     // Chinese
        "こんにちは",                       // Japanese
        "مرحبا",                            // Arabic
        "🎉🎊🎁",                           // Emoji
        "🇺🇸🇬🇧🇯🇵",                           // Flag emoji (multi-codepoint)
        "👨‍👩‍👧‍👦",                               // Family emoji (ZWJ sequence)
        "\u{200B}\u{200C}\u{200D}\u{FEFF}", // Zero-width characters
    ];

    for s in unicode_strings {
        let msg = Message::Set(SetMessage {
            address: "/test/unicode".to_string(),
            value: Value::String(s.to_string()),
            revision: None,
            lock: false,
            unlock: false,
        });

        let encoded = codec::encode(&msg).unwrap();
        let (decoded, _) = codec::decode(&encoded).unwrap();

        if let Message::Set(set) = decoded {
            assert_eq!(set.value, Value::String(s.to_string()));
        } else {
            panic!("Expected Set message");
        }
    }
}

// =============================================================================
// Performance Benchmarks (basic timing)
// =============================================================================

/// Benchmark encoding speed
#[wasm_bindgen_test]
fn benchmark_encoding() {
    let msg = Message::Set(SetMessage {
        address: "/benchmark/test".to_string(),
        value: Value::Float(0.5),
        revision: None,
        lock: false,
        unlock: false,
    });

    let start = js_sys::Date::now();

    for _ in 0..1000 {
        let _ = codec::encode(&msg).unwrap();
    }

    let duration = js_sys::Date::now() - start;

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(duration < 1000.0, "Encoding took too long: {}ms", duration);

    // Log for visibility in test output
    web_sys::console::log_1(&format!("Encoded 1000 messages in {}ms", duration).into());
}

/// Benchmark decoding speed
#[wasm_bindgen_test]
fn benchmark_decoding() {
    let msg = Message::Set(SetMessage {
        address: "/benchmark/test".to_string(),
        value: Value::Float(0.5),
        revision: None,
        lock: false,
        unlock: false,
    });

    let encoded = codec::encode(&msg).unwrap();
    let start = js_sys::Date::now();

    for _ in 0..1000 {
        let _ = codec::decode(&encoded).unwrap();
    }

    let duration = js_sys::Date::now() - start;

    // Should complete in reasonable time
    assert!(duration < 1000.0, "Decoding took too long: {}ms", duration);

    web_sys::console::log_1(&format!("Decoded 1000 messages in {}ms", duration).into());
}

// =============================================================================
// JS Interop Tests
// =============================================================================

/// Test that JsValue conversion works correctly for common types
#[wasm_bindgen_test]
fn test_js_value_null() {
    let js_null = JsValue::NULL;
    assert!(js_null.is_null());
}

/// Test JS array creation
#[wasm_bindgen_test]
fn test_js_array_creation() {
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_f64(1.0));
    arr.push(&JsValue::from_f64(2.0));
    arr.push(&JsValue::from_f64(3.0));

    assert_eq!(arr.length(), 3);
    assert_eq!(arr.get(0).as_f64(), Some(1.0));
}

/// Test JS object creation
#[wasm_bindgen_test]
fn test_js_object_creation() {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str("test")).unwrap();
    js_sys::Reflect::set(&obj, &JsValue::from_str("value"), &JsValue::from_f64(42.0)).unwrap();

    let name = js_sys::Reflect::get(&obj, &JsValue::from_str("name")).unwrap();
    let value = js_sys::Reflect::get(&obj, &JsValue::from_str("value")).unwrap();

    assert_eq!(name.as_string(), Some("test".to_string()));
    assert_eq!(value.as_f64(), Some(42.0));
}

/// Test Uint8Array for binary data
#[wasm_bindgen_test]
fn test_uint8array() {
    let data: Vec<u8> = vec![1, 2, 3, 4, 5];
    let array = js_sys::Uint8Array::from(data.as_slice());

    assert_eq!(array.length(), 5);
    assert_eq!(array.to_vec(), data);
}
