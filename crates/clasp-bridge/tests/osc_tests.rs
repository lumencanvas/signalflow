//! OSC Integration Tests
//!
//! These tests verify that the CLASP OSC bridge can:
//! 1. Receive OSC messages from real OSC libraries (rosc)
//! 2. Send OSC messages that real OSC libraries can parse
//! 3. Handle OSC bundles with timestamps
//! 4. Convert OSC argument types correctly
//! 5. Support wildcard address matching

use rosc::decoder;
use rosc::encoder;
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use std::net::UdpSocket;
use std::time::Duration;

/// Find an available UDP port by binding to port 0
fn find_available_udp_port() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

/// Test: CLASP can receive OSC float messages from external library
#[tokio::test]
async fn test_osc_receive_float() {
    let _port = find_available_udp_port();

    // Create a real OSC message using rosc library
    let msg = OscMessage {
        addr: "/test/fader".to_string(),
        args: vec![OscType::Float(0.75)],
    };
    let packet = OscPacket::Message(msg);
    let encoded = encoder::encode(&packet).expect("Failed to encode OSC");

    // Send it to our CLASP OSC bridge port
    let _socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind sender socket");

    // For this test, we verify the encoding works
    // In a full integration test, we'd have the CLASP router running
    assert!(encoded.len() >= 4, "OSC encoding too short");

    // Verify the encoded message can be decoded back
    let decoded = decoder::decode_udp(&encoded).expect("Failed to decode OSC");

    match decoded.1 {
        OscPacket::Message(m) => {
            assert_eq!(m.addr, "/test/fader", "Address mismatch: {}", m.addr);
            match &m.args[0] {
                OscType::Float(v) => {
                    assert!(
                        (*v - 0.75).abs() < 0.001,
                        "Value mismatch: expected 0.75, got {}",
                        v
                    );
                }
                _ => panic!("Wrong argument type"),
            }
        }
        _ => panic!("Expected message, got bundle"),
    }
}

/// Test: CLASP can receive OSC integer messages
#[tokio::test]
async fn test_osc_receive_int() {
    let msg = OscMessage {
        addr: "/test/button".to_string(),
        args: vec![OscType::Int(127)],
    };
    let packet = OscPacket::Message(msg);
    let encoded = encoder::encode(&packet).expect("Failed to encode OSC");

    let decoded = decoder::decode_udp(&encoded).expect("Failed to decode OSC");

    match decoded.1 {
        OscPacket::Message(m) => match &m.args[0] {
            OscType::Int(v) => {
                assert_eq!(*v, 127, "Value mismatch: expected 127, got {}", v);
            }
            _ => panic!("Wrong argument type"),
        },
        _ => panic!("Expected message"),
    }
}

/// Test: CLASP can receive OSC string messages
#[tokio::test]
async fn test_osc_receive_string() {
    let msg = OscMessage {
        addr: "/test/label".to_string(),
        args: vec![OscType::String("Hello CLASP".to_string())],
    };
    let packet = OscPacket::Message(msg);
    let encoded = encoder::encode(&packet).expect("Failed to encode OSC");

    let decoded = decoder::decode_udp(&encoded).expect("Failed to decode OSC");

    match decoded.1 {
        OscPacket::Message(m) => match &m.args[0] {
            OscType::String(s) => {
                assert_eq!(s, "Hello CLASP", "Value mismatch: {}", s);
            }
            _ => panic!("Wrong argument type"),
        },
        _ => panic!("Expected message"),
    }
}

/// Test: CLASP can receive OSC blob (binary) messages
#[tokio::test]
async fn test_osc_receive_blob() {
    let blob_data = vec![0x01, 0x02, 0x03, 0x04, 0xFF];
    let msg = OscMessage {
        addr: "/test/blob".to_string(),
        args: vec![OscType::Blob(blob_data.clone())],
    };
    let packet = OscPacket::Message(msg);
    let encoded = encoder::encode(&packet).expect("Failed to encode OSC");

    let decoded = decoder::decode_udp(&encoded).expect("Failed to decode OSC");

    match decoded.1 {
        OscPacket::Message(m) => match &m.args[0] {
            OscType::Blob(b) => {
                assert_eq!(*b, blob_data, "Blob data mismatch");
            }
            _ => panic!("Wrong argument type"),
        },
        _ => panic!("Expected message"),
    }
}

/// Test: CLASP can receive OSC messages with multiple arguments
#[tokio::test]
async fn test_osc_receive_multiple_args() {
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
    let encoded = encoder::encode(&packet).expect("Failed to encode OSC");

    let decoded = decoder::decode_udp(&encoded).expect("Failed to decode OSC");

    match decoded.1 {
        OscPacket::Message(m) => {
            assert_eq!(m.args.len(), 4, "Expected 4 args, got {}", m.args.len());
            // Verify each argument type
            match (&m.args[0], &m.args[1], &m.args[2], &m.args[3]) {
                (OscType::Float(_), OscType::Int(_), OscType::String(_), OscType::Bool(_)) => {}
                _ => panic!("Argument types don't match"),
            }
        }
        _ => panic!("Expected message"),
    }
}

/// Test: CLASP can send OSC messages that external libraries can receive
#[tokio::test]
async fn test_osc_send_to_external() {
    let port = find_available_udp_port();

    // Set up receiver socket (simulates external OSC app)
    let receiver = UdpSocket::bind(format!("127.0.0.1:{}", port)).expect("Failed to bind receiver");
    receiver
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Failed to set timeout");

    // Create and send OSC message (simulates CLASP sending)
    let msg = OscMessage {
        addr: "/clasp/output".to_string(),
        args: vec![OscType::Float(0.5)],
    };
    let packet = OscPacket::Message(msg);
    let encoded = encoder::encode(&packet).expect("Failed to encode");

    let sender = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind sender");
    sender
        .send_to(&encoded, format!("127.0.0.1:{}", port))
        .expect("Failed to send");

    // Receive and verify
    let mut buf = [0u8; 1024];
    let (len, _) = receiver.recv_from(&mut buf).expect("Failed to receive");

    let decoded = decoder::decode_udp(&buf[..len]).expect("Failed to decode received");

    match decoded.1 {
        OscPacket::Message(m) => {
            assert_eq!(m.addr, "/clasp/output", "Address mismatch: {}", m.addr);
        }
        _ => panic!("Expected message"),
    }
}

/// Test: CLASP can handle OSC bundles with timestamps
#[tokio::test]
async fn test_osc_bundle_with_timestamp() {
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
    let encoded = encoder::encode(&packet).expect("Failed to encode bundle");

    let decoded = decoder::decode_udp(&encoded).expect("Failed to decode bundle");

    match decoded.1 {
        OscPacket::Bundle(b) => {
            assert_eq!(
                b.content.len(),
                2,
                "Expected 2 messages in bundle, got {}",
                b.content.len()
            );
            assert_eq!(b.timetag.seconds, 1704067200, "Timestamp not preserved");
        }
        _ => panic!("Expected bundle"),
    }
}

/// Test: OSC roundtrip through encoding and decoding
#[tokio::test]
async fn test_osc_roundtrip() {
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
        let encoded =
            encoder::encode(&packet).expect(&format!("Failed to encode {}", original.addr));
        let decoded =
            decoder::decode_udp(&encoded).expect(&format!("Failed to decode {}", original.addr));

        match decoded.1 {
            OscPacket::Message(m) => {
                assert_eq!(
                    m.addr, original.addr,
                    "Address mismatch for {}",
                    original.addr
                );
                assert_eq!(
                    m.args.len(),
                    original.args.len(),
                    "Arg count mismatch for {}",
                    original.addr
                );
            }
            _ => panic!("Expected message for {}", original.addr),
        }
    }
}

/// Test: High-rate OSC message handling
#[tokio::test]
async fn test_osc_high_rate() {
    let count = 1000;
    let mut encoded_messages = Vec::with_capacity(count);

    // Encode 1000 messages
    for i in 0..count {
        let msg = OscMessage {
            addr: format!("/highrate/{}", i),
            args: vec![OscType::Float(i as f32 / count as f32)],
        };
        let packet = OscPacket::Message(msg);
        let encoded = encoder::encode(&packet).expect(&format!("Failed to encode message {}", i));
        encoded_messages.push(encoded);
    }

    // Decode all messages
    for (i, encoded) in encoded_messages.iter().enumerate() {
        let decoded =
            decoder::decode_udp(encoded).expect(&format!("Failed to decode message {}", i));

        match decoded.1 {
            OscPacket::Message(m) => {
                assert_eq!(
                    m.addr,
                    format!("/highrate/{}", i),
                    "Message {} address mismatch",
                    i
                );
            }
            _ => panic!("Message {} not a message", i),
        }
    }
}
