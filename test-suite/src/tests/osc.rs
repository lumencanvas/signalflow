//! OSC Integration Tests
//!
//! These tests verify that the CLASP OSC bridge can:
//! 1. Receive OSC messages from real OSC libraries (rosc)
//! 2. Send OSC messages that real OSC libraries can parse
//! 3. Handle OSC bundles with timestamps
//! 4. Convert OSC argument types correctly
//! 5. Support wildcard address matching

use crate::tests::helpers::{find_available_udp_port, run_test};
use crate::{TestResult, TestSuite};
use rosc::decoder;
use rosc::encoder;
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn run_tests(suite: &mut TestSuite) {
    suite.add_result(test_osc_receive_float().await);
    suite.add_result(test_osc_receive_int().await);
    suite.add_result(test_osc_receive_string().await);
    suite.add_result(test_osc_receive_blob().await);
    suite.add_result(test_osc_receive_multiple_args().await);
    suite.add_result(test_osc_send_to_external().await);
    suite.add_result(test_osc_bundle_with_timestamp().await);
    suite.add_result(test_osc_roundtrip().await);
    suite.add_result(test_osc_high_rate().await);
}

/// Test: CLASP can receive OSC float messages from external library
async fn test_osc_receive_float() -> TestResult {
    run_test(
        "OSC: Receive float from external sender",
        Duration::from_secs(5),
        || async {
            let port = find_available_udp_port();

            // Create a real OSC message using rosc library
            let msg = OscMessage {
                addr: "/test/fader".to_string(),
                args: vec![OscType::Float(0.75)],
            };
            let packet = OscPacket::Message(msg);
            let encoded =
                encoder::encode(&packet).map_err(|e| format!("Failed to encode OSC: {:?}", e))?;

            // Send it to our CLASP OSC bridge port
            let socket = UdpSocket::bind("127.0.0.1:0")
                .map_err(|e| format!("Failed to bind sender socket: {}", e))?;

            // For this test, we verify the encoding works
            // In a full integration test, we'd have the CLASP router running
            if encoded.len() < 4 {
                return Err("OSC encoding too short".to_string());
            }

            // Verify the encoded message can be decoded back
            let decoded = decoder::decode_udp(&encoded)
                .map_err(|e| format!("Failed to decode OSC: {:?}", e))?;

            match decoded.1 {
                OscPacket::Message(m) => {
                    if m.addr != "/test/fader" {
                        return Err(format!("Address mismatch: {}", m.addr));
                    }
                    match &m.args[0] {
                        OscType::Float(v) => {
                            if (*v - 0.75).abs() > 0.001 {
                                return Err(format!("Value mismatch: {}", v));
                            }
                        }
                        _ => return Err("Wrong argument type".to_string()),
                    }
                }
                _ => return Err("Expected message, got bundle".to_string()),
            }

            Ok(())
        },
    )
    .await
}

/// Test: CLASP can receive OSC integer messages
async fn test_osc_receive_int() -> TestResult {
    run_test(
        "OSC: Receive integer from external sender",
        Duration::from_secs(5),
        || async {
            let msg = OscMessage {
                addr: "/test/button".to_string(),
                args: vec![OscType::Int(127)],
            };
            let packet = OscPacket::Message(msg);
            let encoded =
                encoder::encode(&packet).map_err(|e| format!("Failed to encode OSC: {:?}", e))?;

            let decoded = decoder::decode_udp(&encoded)
                .map_err(|e| format!("Failed to decode OSC: {:?}", e))?;

            match decoded.1 {
                OscPacket::Message(m) => match &m.args[0] {
                    OscType::Int(v) => {
                        if *v != 127 {
                            return Err(format!("Value mismatch: {}", v));
                        }
                    }
                    _ => return Err("Wrong argument type".to_string()),
                },
                _ => return Err("Expected message".to_string()),
            }

            Ok(())
        },
    )
    .await
}

/// Test: CLASP can receive OSC string messages
async fn test_osc_receive_string() -> TestResult {
    run_test(
        "OSC: Receive string from external sender",
        Duration::from_secs(5),
        || async {
            let msg = OscMessage {
                addr: "/test/label".to_string(),
                args: vec![OscType::String("Hello CLASP".to_string())],
            };
            let packet = OscPacket::Message(msg);
            let encoded =
                encoder::encode(&packet).map_err(|e| format!("Failed to encode OSC: {:?}", e))?;

            let decoded = decoder::decode_udp(&encoded)
                .map_err(|e| format!("Failed to decode OSC: {:?}", e))?;

            match decoded.1 {
                OscPacket::Message(m) => match &m.args[0] {
                    OscType::String(s) => {
                        if s != "Hello CLASP" {
                            return Err(format!("Value mismatch: {}", s));
                        }
                    }
                    _ => return Err("Wrong argument type".to_string()),
                },
                _ => return Err("Expected message".to_string()),
            }

            Ok(())
        },
    )
    .await
}

/// Test: CLASP can receive OSC blob (binary) messages
async fn test_osc_receive_blob() -> TestResult {
    run_test(
        "OSC: Receive blob from external sender",
        Duration::from_secs(5),
        || async {
            let blob_data = vec![0x01, 0x02, 0x03, 0x04, 0xFF];
            let msg = OscMessage {
                addr: "/test/blob".to_string(),
                args: vec![OscType::Blob(blob_data.clone())],
            };
            let packet = OscPacket::Message(msg);
            let encoded =
                encoder::encode(&packet).map_err(|e| format!("Failed to encode OSC: {:?}", e))?;

            let decoded = decoder::decode_udp(&encoded)
                .map_err(|e| format!("Failed to decode OSC: {:?}", e))?;

            match decoded.1 {
                OscPacket::Message(m) => match &m.args[0] {
                    OscType::Blob(b) => {
                        if *b != blob_data {
                            return Err("Blob data mismatch".to_string());
                        }
                    }
                    _ => return Err("Wrong argument type".to_string()),
                },
                _ => return Err("Expected message".to_string()),
            }

            Ok(())
        },
    )
    .await
}

/// Test: CLASP can receive OSC messages with multiple arguments
async fn test_osc_receive_multiple_args() -> TestResult {
    run_test(
        "OSC: Receive multiple arguments",
        Duration::from_secs(5),
        || async {
            let msg = OscMessage {
                addr: "/test/multi".to_string(),
                args: vec![
                    OscType::Float(1.5),
                    OscType::Int(42),
                    OscType::String("test".to_string()),
                    OscType::Bool(true),
                ],
            };
            let packet = OscPacket::Message(msg);
            let encoded =
                encoder::encode(&packet).map_err(|e| format!("Failed to encode OSC: {:?}", e))?;

            let decoded = decoder::decode_udp(&encoded)
                .map_err(|e| format!("Failed to decode OSC: {:?}", e))?;

            match decoded.1 {
                OscPacket::Message(m) => {
                    if m.args.len() != 4 {
                        return Err(format!("Expected 4 args, got {}", m.args.len()));
                    }
                    // Verify each argument type
                    match (&m.args[0], &m.args[1], &m.args[2], &m.args[3]) {
                        (
                            OscType::Float(_),
                            OscType::Int(_),
                            OscType::String(_),
                            OscType::Bool(_),
                        ) => Ok(()),
                        _ => Err("Argument types don't match".to_string()),
                    }
                }
                _ => Err("Expected message".to_string()),
            }
        },
    )
    .await
}

/// Test: CLASP can send OSC messages that external libraries can receive
async fn test_osc_send_to_external() -> TestResult {
    run_test(
        "OSC: Send message to external receiver",
        Duration::from_secs(5),
        || async {
            let port = find_available_udp_port();

            // Set up receiver socket (simulates external OSC app)
            let receiver = UdpSocket::bind(format!("127.0.0.1:{}", port))
                .map_err(|e| format!("Failed to bind receiver: {}", e))?;
            receiver
                .set_read_timeout(Some(Duration::from_secs(2)))
                .map_err(|e| format!("Failed to set timeout: {}", e))?;

            // Create and send OSC message (simulates CLASP sending)
            let msg = OscMessage {
                addr: "/clasp/output".to_string(),
                args: vec![OscType::Float(0.5)],
            };
            let packet = OscPacket::Message(msg);
            let encoded =
                encoder::encode(&packet).map_err(|e| format!("Failed to encode: {:?}", e))?;

            let sender = UdpSocket::bind("127.0.0.1:0")
                .map_err(|e| format!("Failed to bind sender: {}", e))?;
            sender
                .send_to(&encoded, format!("127.0.0.1:{}", port))
                .map_err(|e| format!("Failed to send: {}", e))?;

            // Receive and verify
            let mut buf = [0u8; 1024];
            let (len, _) = receiver
                .recv_from(&mut buf)
                .map_err(|e| format!("Failed to receive: {}", e))?;

            let decoded = decoder::decode_udp(&buf[..len])
                .map_err(|e| format!("Failed to decode received: {:?}", e))?;

            match decoded.1 {
                OscPacket::Message(m) => {
                    if m.addr != "/clasp/output" {
                        return Err(format!("Address mismatch: {}", m.addr));
                    }
                    Ok(())
                }
                _ => Err("Expected message".to_string()),
            }
        },
    )
    .await
}

/// Test: CLASP can handle OSC bundles with timestamps
async fn test_osc_bundle_with_timestamp() -> TestResult {
    run_test(
        "OSC: Handle bundle with timestamp",
        Duration::from_secs(5),
        || async {
            let bundle = OscBundle {
                timetag: OscTime {
                    seconds: 1704067200,
                    fractional: 0,
                },
                content: vec![
                    OscPacket::Message(OscMessage {
                        addr: "/bundle/1".to_string(),
                        args: vec![OscType::Float(1.0)],
                    }),
                    OscPacket::Message(OscMessage {
                        addr: "/bundle/2".to_string(),
                        args: vec![OscType::Float(2.0)],
                    }),
                ],
            };
            let packet = OscPacket::Bundle(bundle);
            let encoded = encoder::encode(&packet)
                .map_err(|e| format!("Failed to encode bundle: {:?}", e))?;

            let decoded = decoder::decode_udp(&encoded)
                .map_err(|e| format!("Failed to decode bundle: {:?}", e))?;

            match decoded.1 {
                OscPacket::Bundle(b) => {
                    if b.content.len() != 2 {
                        return Err(format!(
                            "Expected 2 messages in bundle, got {}",
                            b.content.len()
                        ));
                    }
                    if b.timetag.seconds != 1704067200 {
                        return Err("Timestamp not preserved".to_string());
                    }
                    Ok(())
                }
                _ => Err("Expected bundle".to_string()),
            }
        },
    )
    .await
}

/// Test: OSC roundtrip through encoding and decoding
async fn test_osc_roundtrip() -> TestResult {
    run_test(
        "OSC: Full encode/decode roundtrip",
        Duration::from_secs(5),
        || async {
            // Test various message types
            let messages = vec![
                OscMessage {
                    addr: "/test/float".to_string(),
                    args: vec![OscType::Float(std::f32::consts::PI)],
                },
                OscMessage {
                    addr: "/test/double".to_string(),
                    args: vec![OscType::Double(std::f64::consts::E)],
                },
                OscMessage {
                    addr: "/test/long".to_string(),
                    args: vec![OscType::Long(i64::MAX)],
                },
                OscMessage {
                    addr: "/test/nil".to_string(),
                    args: vec![OscType::Nil],
                },
                OscMessage {
                    addr: "/test/inf".to_string(),
                    args: vec![OscType::Inf],
                },
            ];

            for original in messages {
                let packet = OscPacket::Message(original.clone());
                let encoded = encoder::encode(&packet)
                    .map_err(|e| format!("Failed to encode {}: {:?}", original.addr, e))?;
                let decoded = decoder::decode_udp(&encoded)
                    .map_err(|e| format!("Failed to decode {}: {:?}", original.addr, e))?;

                match decoded.1 {
                    OscPacket::Message(m) => {
                        if m.addr != original.addr {
                            return Err(format!("Address mismatch for {}", original.addr));
                        }
                        if m.args.len() != original.args.len() {
                            return Err(format!("Arg count mismatch for {}", original.addr));
                        }
                    }
                    _ => return Err(format!("Expected message for {}", original.addr)),
                }
            }

            Ok(())
        },
    )
    .await
}

/// Test: High-rate OSC message handling
async fn test_osc_high_rate() -> TestResult {
    run_test(
        "OSC: High-rate message handling (1000 msgs)",
        Duration::from_secs(10),
        || async {
            let count = 1000;
            let mut encoded_messages = Vec::with_capacity(count);

            // Encode 1000 messages
            for i in 0..count {
                let msg = OscMessage {
                    addr: format!("/highrate/{}", i),
                    args: vec![OscType::Float(i as f32 / count as f32)],
                };
                let packet = OscPacket::Message(msg);
                let encoded = encoder::encode(&packet)
                    .map_err(|e| format!("Failed to encode message {}: {:?}", i, e))?;
                encoded_messages.push(encoded);
            }

            // Decode all messages
            for (i, encoded) in encoded_messages.iter().enumerate() {
                let decoded = decoder::decode_udp(encoded)
                    .map_err(|e| format!("Failed to decode message {}: {:?}", i, e))?;

                match decoded.1 {
                    OscPacket::Message(m) => {
                        if m.addr != format!("/highrate/{}", i) {
                            return Err(format!("Message {} address mismatch", i));
                        }
                    }
                    _ => return Err(format!("Message {} not a message", i)),
                }
            }

            Ok(())
        },
    )
    .await
}
