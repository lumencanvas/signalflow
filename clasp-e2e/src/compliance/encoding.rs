//! Encoding Conformance Tests
//!
//! Tests for CLASP binary encoding format (CLASP Spec 2.x):
//! - Message framing
//! - Value type encoding
//! - Address encoding
//! - Payload structure

use super::{ConformanceConfig, ConformanceReport, TestResult};
use anyhow;
use clasp_core::{codec, HelloMessage, Message, SetMessage, Value, PROTOCOL_VERSION};
use std::time::Instant;

pub async fn run_tests(config: &ConformanceConfig, report: &mut ConformanceReport) {
    test_hello_encoding(config, report).await;
    test_set_encoding(config, report).await;
    test_value_int_encoding(config, report).await;
    test_value_float_encoding(config, report).await;
    test_value_string_encoding(config, report).await;
    test_value_bool_encoding(config, report).await;
    test_value_bytes_encoding(config, report).await;
    test_roundtrip_encoding(config, report).await;
}

async fn test_hello_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "HELLO message encoding";

    let result = (|| {
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "Encoding Test".to_string(),
            features: vec!["param".to_string(), "event".to_string()],
            capabilities: None,
            token: None,
        });

        // Encode
        let encoded = codec::encode(&hello)?;

        // Verify non-empty
        if encoded.is_empty() {
            return Err(anyhow::anyhow!("Encoded HELLO is empty"));
        }

        // Decode
        let (decoded, _) = codec::decode(&encoded)?;

        // Verify roundtrip
        match decoded {
            Message::Hello(h) => {
                if h.version != PROTOCOL_VERSION {
                    return Err(anyhow::anyhow!("Version mismatch after decode"));
                }
                if h.name != "Encoding Test" {
                    return Err(anyhow::anyhow!("Name mismatch after decode"));
                }
                if h.features.len() != 2 {
                    return Err(anyhow::anyhow!("Features count mismatch"));
                }
            }
            _ => return Err(anyhow::anyhow!("Decoded wrong message type")),
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.1"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.1"),
        ),
    }
}

async fn test_set_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "SET message encoding";

    let result = (|| {
        let set = Message::Set(SetMessage {
            address: "/test/address".to_string(),
            value: Value::Int(42),
            revision: Some(1),
            lock: false,
            unlock: false,
        });

        let encoded = codec::encode(&set)?;

        if encoded.is_empty() {
            return Err(anyhow::anyhow!("Encoded SET is empty"));
        }

        let (decoded, _) = codec::decode(&encoded)?;

        match decoded {
            Message::Set(s) => {
                if s.address != "/test/address" {
                    return Err(anyhow::anyhow!("Address mismatch"));
                }
                match s.value {
                    Value::Int(v) => {
                        if v != 42 {
                            return Err(anyhow::anyhow!("Value mismatch"));
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Value type mismatch")),
                }
            }
            _ => return Err(anyhow::anyhow!("Wrong message type")),
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.2"),
        ),
    }
}

async fn test_value_int_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Int value encoding";

    let result = (|| {
        let test_values = vec![
            0i64,
            1,
            -1,
            127,
            -128,
            32767,
            -32768,
            i32::MAX as i64,
            i32::MIN as i64,
            i64::MAX,
            i64::MIN,
        ];

        for val in test_values {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: Value::Int(val),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = codec::encode(&msg)?;
            let (decoded, _) = codec::decode(&encoded)?;

            match decoded {
                Message::Set(s) => match s.value {
                    Value::Int(v) => {
                        if v != val {
                            return Err(anyhow::anyhow!("Int {} encoded/decoded as {}", val, v));
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Int became different type")),
                },
                _ => return Err(anyhow::anyhow!("Wrong message type")),
            }
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.3.1"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.3.1"),
        ),
    }
}

async fn test_value_float_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Float value encoding";

    let result = (|| {
        let test_values = vec![
            0.0f64,
            1.0,
            -1.0,
            3.14159265358979,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::MIN,
        ];

        for val in test_values {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: Value::Float(val),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = codec::encode(&msg)?;
            let (decoded, _) = codec::decode(&encoded)?;

            match decoded {
                Message::Set(s) => match s.value {
                    Value::Float(v) => {
                        if (v - val).abs() > f64::EPSILON {
                            return Err(anyhow::anyhow!("Float {} encoded/decoded as {}", val, v));
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Float became different type")),
                },
                _ => return Err(anyhow::anyhow!("Wrong message type")),
            }
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.3.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.3.2"),
        ),
    }
}

async fn test_value_string_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "String value encoding";

    let result = (|| {
        let long_string = "a".repeat(1000);
        let test_values = vec![
            "",
            "hello",
            "Hello, World!",
            "unicode: 日本語 🎵 émojis",
            &long_string, // Long string
        ];

        for val in test_values {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: Value::String(val.to_string()),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = codec::encode(&msg)?;
            let (decoded, _) = codec::decode(&encoded)?;

            match decoded {
                Message::Set(s) => match s.value {
                    Value::String(v) => {
                        if v != val {
                            return Err(anyhow::anyhow!(
                                "String '{}' encoded/decoded as '{}'",
                                val,
                                v
                            ));
                        }
                    }
                    _ => return Err(anyhow::anyhow!("String became different type")),
                },
                _ => return Err(anyhow::anyhow!("Wrong message type")),
            }
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.3.3"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.3.3"),
        ),
    }
}

async fn test_value_bool_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Bool value encoding";

    let result = (|| {
        for val in [true, false] {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: Value::Bool(val),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = codec::encode(&msg)?;
            let (decoded, _) = codec::decode(&encoded)?;

            match decoded {
                Message::Set(s) => match s.value {
                    Value::Bool(v) => {
                        if v != val {
                            return Err(anyhow::anyhow!("Bool {} encoded/decoded as {}", val, v));
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Bool became different type")),
                },
                _ => return Err(anyhow::anyhow!("Wrong message type")),
            }
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.3.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.3.4"),
        ),
    }
}

async fn test_value_bytes_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Bytes value encoding";

    let result = (|| {
        let test_values: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![0, 1, 2, 3, 4, 5],
            vec![255; 100],
            (0u8..=255).collect(),
        ];

        for val in test_values {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: Value::Bytes(val.clone()),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = codec::encode(&msg)?;
            let (decoded, _) = codec::decode(&encoded)?;

            match decoded {
                Message::Set(s) => match s.value {
                    Value::Bytes(v) => {
                        if v != val {
                            return Err(anyhow::anyhow!(
                                "Bytes of len {} encoded/decoded as len {}",
                                val.len(),
                                v.len()
                            ));
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Bytes became different type")),
                },
                _ => return Err(anyhow::anyhow!("Wrong message type")),
            }
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.3.5"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.3.5"),
        ),
    }
}

async fn test_roundtrip_encoding(_config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Full roundtrip encoding";

    let result = (|| {
        // Test all message types can roundtrip
        let messages = vec![
            Message::Hello(HelloMessage {
                version: PROTOCOL_VERSION,
                name: "Test".to_string(),
                features: vec![],
                capabilities: None,
                token: Some("token".to_string()),
            }),
            Message::Set(SetMessage {
                address: "/a/b/c".to_string(),
                value: Value::Int(123),
                revision: Some(5),
                lock: true,
                unlock: false,
            }),
        ];

        for msg in messages {
            let encoded = codec::encode(&msg)?;
            let (decoded, _frame) = codec::decode(&encoded)?;

            // Re-encode and compare
            let re_encoded = codec::encode(&decoded)?;
            if encoded != re_encoded {
                return Err(anyhow::anyhow!("Re-encoded bytes differ"));
            }
        }

        Ok(())
    })();

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Encoding", duration).with_spec_reference("CLASP 2.x"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Encoding", duration, &e.to_string())
                .with_spec_reference("CLASP 2.x"),
        ),
    }
}
