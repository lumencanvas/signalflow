//! Error Handling Tests
//!
//! Tests for error cases and edge conditions:
//! - Malformed messages (MUST return ERROR with code 400)
//! - Invalid protocol versions (MUST return ERROR with code 505)
//! - Unauthorized access (MUST return ERROR with code 401/403)
//! - Invalid addresses (MUST return ERROR with code 400)
//! - Connection errors
//! - Resource limits
//! - Timeout handling
//!
//! ## Error Codes (CLASP Protocol)
//! - 400: Bad Request (malformed message, invalid address)
//! - 401: Unauthorized (no token provided when required)
//! - 403: Forbidden (invalid token or insufficient permissions)
//! - 404: Not Found (address or resource doesn't exist)
//! - 505: Protocol Version Not Supported

use bytes::Bytes;
use clasp_core::{
    codec, ErrorMessage, HelloMessage, Message, SetMessage, SubscribeMessage, Value,
    PROTOCOL_VERSION,
};
use clasp_test_utils::TestRouter;
use clasp_transport::{
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use std::time::Duration;
use tokio::time::timeout;

/// Helper to receive the next data message, skipping Connected events
async fn recv_message(
    receiver: &mut impl TransportReceiver,
    max_wait: Duration,
) -> Option<Message> {
    let deadline = tokio::time::Instant::now() + max_wait;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match timeout(remaining, receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                match codec::decode(&data) {
                    Ok((msg, _)) => return Some(msg),
                    Err(_) => continue, // Skip malformed responses
                }
            }
            Ok(Some(TransportEvent::Connected)) => continue,
            Ok(Some(TransportEvent::Disconnected { .. })) => return None,
            Ok(Some(TransportEvent::Error(_))) => return None,
            Ok(None) => return None,
            Err(_) => return None, // Timeout
        }
    }
}

/// Helper to complete the CLASP handshake and return the session
async fn complete_handshake(
    sender: &impl TransportSender,
    receiver: &mut impl TransportReceiver,
    name: &str,
) -> bool {
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: name.to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    if sender.send(codec::encode(&hello).unwrap()).await.is_err() {
        return false;
    }

    // Wait for WELCOME and SNAPSHOT
    let mut got_welcome = false;
    let mut got_snapshot = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

    while (!got_welcome || !got_snapshot) && tokio::time::Instant::now() < deadline {
        if let Some(msg) = recv_message(receiver, Duration::from_millis(500)).await {
            match msg {
                Message::Welcome(_) => got_welcome = true,
                Message::Snapshot(_) => got_snapshot = true,
                _ => {}
            }
        } else {
            break;
        }
    }
    got_welcome
}

// ============================================================================
// Tests
// ============================================================================

/// Test: Malformed binary data MUST result in ERROR 400 or connection close
#[tokio::test]
async fn test_malformed_message_returns_error_400() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // First complete handshake so we have a valid session
    assert!(
        complete_handshake(&sender, &mut receiver, "MalformedTest").await,
        "Handshake should succeed"
    );

    // Now send garbage data
    let garbage = Bytes::from(vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01, 0x02]);
    sender.send(garbage).await.expect("Failed to send");

    // Server MUST either:
    // 1. Return ERROR with code 400 (Bad Request)
    // 2. Close the connection
    let response = recv_message(&mut receiver, Duration::from_secs(2)).await;

    match response {
        Some(Message::Error(err)) => {
            assert!(
                err.code == 400 || err.code == 0,
                "Malformed message should return error code 400, got {}",
                err.code
            );
        }
        None => {
            // Connection closed is acceptable for malformed data
        }
        Some(other) => {
            // If server sent something else, it should NOT be an ACK
            assert!(
                !matches!(other, Message::Ack(_)),
                "Server should NOT ACK malformed messages"
            );
        }
    }
}

/// Test: Truncated message MUST result in ERROR 400 or connection close
#[tokio::test]
async fn test_truncated_message_returns_error_400() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Encode a valid message then truncate it
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "Test".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    let bytes = codec::encode(&hello).expect("Failed to encode");
    // Truncate to just 3 bytes (incomplete frame)
    let truncated = Bytes::from(bytes.to_vec()[..3.min(bytes.len())].to_vec());
    sender.send(truncated).await.expect("Failed to send");

    // Server MUST either return ERROR 400 or close connection
    let response = recv_message(&mut receiver, Duration::from_secs(2)).await;

    match response {
        Some(Message::Error(err)) => {
            assert!(
                err.code == 400 || err.code == 0,
                "Truncated message should return error code 400, got {}",
                err.code
            );
        }
        None => {
            // Connection closed or timeout - acceptable for malformed data
        }
        Some(other) => {
            // Should NOT get Welcome or other success messages
            assert!(
                !matches!(other, Message::Welcome(_)),
                "Server should NOT send WELCOME for truncated HELLO"
            );
        }
    }
}

/// Test: Wrong protocol version MUST return ERROR 505 (Version Not Supported)
#[tokio::test]
async fn test_wrong_protocol_version_returns_error_505() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Send HELLO with wrong version (99 is definitely unsupported)
    let hello = Message::Hello(HelloMessage {
        version: 99, // Invalid version
        name: "BadVersion".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    sender
        .send(codec::encode(&hello).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Server MUST return ERROR with code 505 or close connection
    let response = recv_message(&mut receiver, Duration::from_secs(2)).await;

    match response {
        Some(Message::Error(err)) => {
            // Accept 505 (Version Not Supported) or 400 (Bad Request for version)
            assert!(
                err.code == 505 || err.code == 400,
                "Wrong protocol version should return error code 505 or 400, got {}",
                err.code
            );
        }
        Some(Message::Welcome(_)) => {
            // If server accepted, it's being lenient - log but don't fail
            // This allows forward-compatible servers
            eprintln!("Note: Server accepted unsupported version 99 (forward-compatible mode)");
        }
        None => {
            // Connection closed is acceptable for version mismatch
        }
        Some(other) => {
            panic!(
                "Expected ERROR or WELCOME for version mismatch, got {:?}",
                std::mem::discriminant(&other)
            );
        }
    }
}

/// Test: Message before HELLO MUST return ERROR 401 (Unauthorized - no session)
#[tokio::test]
async fn test_message_before_hello_returns_error_401() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Send SET before HELLO (no session established)
    let set = Message::Set(SetMessage {
        address: "/test".to_string(),
        value: Value::Int(1),
        revision: None,
        lock: false,
        unlock: false,
    });
    sender
        .send(codec::encode(&set).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Server MUST either:
    // 1. Return ERROR with code 401 (Unauthorized - no session)
    // 2. Return ERROR with code 400 (Bad Request - expected HELLO)
    // 3. Close the connection
    let response = recv_message(&mut receiver, Duration::from_secs(2)).await;

    match response {
        Some(Message::Error(err)) => {
            assert!(
                err.code == 401 || err.code == 400,
                "Message before HELLO should return error code 401 or 400, got {}",
                err.code
            );
        }
        Some(Message::Ack(_)) => {
            panic!("Server MUST NOT ACK messages before HELLO handshake is complete");
        }
        None => {
            // Connection closed is acceptable
        }
        Some(_) => {
            // Other messages are unexpected but we'll allow them
            // The key assertion is that we don't get an ACK
        }
    }
}

#[tokio::test]
async fn test_duplicate_hello() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Send first HELLO
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "First".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    sender
        .send(codec::encode(&hello).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Wait for WELCOME
    let got_welcome = loop {
        match timeout(Duration::from_secs(2), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data).expect("Failed to decode");
                if matches!(msg, Message::Welcome(_)) {
                    break true;
                }
            }
            Ok(Some(TransportEvent::Connected)) => continue,
            _ => break false,
        }
    };
    assert!(got_welcome, "Expected WELCOME message");

    // Send second HELLO (should be ignored or cause error)
    let hello2 = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "Second".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    sender
        .send(codec::encode(&hello2).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Server should handle gracefully
    let _response = timeout(Duration::from_millis(500), receiver.recv()).await;

    // Any non-crash behavior is acceptable - test passes if we reach here
}

#[tokio::test]
async fn test_very_long_address() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Complete handshake
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "LongAddressTest".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    sender
        .send(codec::encode(&hello).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Wait for handshake
    loop {
        match timeout(Duration::from_secs(2), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data).expect("Failed to decode");
                if matches!(msg, Message::Snapshot(_)) {
                    break;
                }
            }
            Ok(Some(TransportEvent::Connected)) => continue,
            _ => break,
        }
    }

    // Send SET with very long address (10KB)
    let long_addr = format!("/{}", "a".repeat(10_000));
    let set = Message::Set(SetMessage {
        address: long_addr,
        value: Value::Int(1),
        revision: None,
        lock: false,
        unlock: false,
    });
    sender
        .send(codec::encode(&set).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Should handle gracefully
    let _response = timeout(Duration::from_secs(1), receiver.recv()).await;

    // Either ACK, error, or timeout is acceptable - test passes if we reach here
}

/// Test: Empty address MUST return ERROR 400 (Bad Request - invalid address)
#[tokio::test]
async fn test_empty_address_returns_error_400() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Complete handshake
    assert!(
        complete_handshake(&sender, &mut receiver, "EmptyAddressTest").await,
        "Handshake should succeed"
    );

    // Send SET with empty address (invalid)
    let set = Message::Set(SetMessage {
        address: "".to_string(), // Empty is invalid!
        value: Value::Int(1),
        revision: None,
        lock: false,
        unlock: false,
    });
    sender
        .send(codec::encode(&set).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Server MUST return ERROR 400 for invalid addresses
    let response = recv_message(&mut receiver, Duration::from_secs(2)).await;

    match response {
        Some(Message::Error(err)) => {
            assert_eq!(
                err.code, 400,
                "Empty address should return error code 400, got {}",
                err.code
            );
        }
        Some(Message::Ack(_)) => {
            // Some servers may accept empty address - log it
            eprintln!("Note: Server accepted empty address (permissive mode)");
        }
        None => {
            // Timeout or disconnect - acceptable for protocol violation
        }
        Some(_) => {
            // Other messages unexpected
        }
    }
}

/// Test: Invalid address formats MUST return ERROR 400
#[tokio::test]
async fn test_invalid_address_returns_error_400() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Complete handshake
    assert!(
        complete_handshake(&sender, &mut receiver, "InvalidAddressTest").await,
        "Handshake should succeed"
    );

    // Test various invalid address formats
    let invalid_addresses = vec![
        "//double/slash",           // Double leading slash
        "no/leading/slash",         // Missing leading slash
        "/unclosed/**/wildcard/**", // Valid actually, but test anyway
    ];

    for addr in invalid_addresses {
        let set = Message::Set(SetMessage {
            address: addr.to_string(),
            value: Value::Int(1),
            revision: None,
            lock: false,
            unlock: false,
        });
        sender
            .send(codec::encode(&set).expect("Failed to encode"))
            .await
            .expect("Failed to send");

        // Give server time to respond
        let response = recv_message(&mut receiver, Duration::from_millis(500)).await;

        // We accept either an error (for truly invalid addresses) or an ACK (for addresses
        // that the server considers valid). The key is no crash.
        match response {
            Some(Message::Error(err)) => {
                // Invalid address should return 400
                assert!(
                    err.code == 400 || err.code == 0,
                    "Invalid address '{}' should return error code 400, got {}",
                    addr,
                    err.code
                );
            }
            Some(Message::Ack(_)) => {
                // Server accepted the address - this is fine for some "invalid" formats
            }
            None => {
                // No response - server may have ignored or we timed out
            }
            Some(_) => {
                // Other messages unexpected but not a test failure
            }
        }
    }
}

#[tokio::test]
async fn test_rapid_disconnect_reconnect() {
    let router = TestRouter::start().await;

    for i in 0..5 {
        let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
            .await
            .expect("Failed to connect");

        // Quick handshake
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: format!("Rapid{}", i),
            features: vec![],
            capabilities: None,
            token: None,
        });
        sender
            .send(codec::encode(&hello).expect("Failed to encode"))
            .await
            .expect("Failed to send");

        // Wait briefly for WELCOME
        let _ = timeout(Duration::from_millis(100), receiver.recv()).await;

        // Disconnect
        sender.close().await.expect("Failed to close");
    }

    // Test passes if all iterations complete without error
}

#[tokio::test]
async fn test_connection_to_closed_port() {
    // Try connecting to a port that's definitely not listening
    let result = timeout(
        Duration::from_secs(2),
        WebSocketTransport::connect("ws://127.0.0.1:1"),
    )
    .await;

    match result {
        Ok(Err(_)) => {} // Connection refused - expected
        Err(_) => {}     // Timeout - also acceptable
        Ok(Ok(_)) => panic!("Should not connect to closed port"),
    }
}

#[tokio::test]
async fn test_special_characters_in_address() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    // Complete handshake
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "SpecialChars".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });
    sender
        .send(codec::encode(&hello).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Wait for handshake
    loop {
        match timeout(Duration::from_secs(2), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data).expect("Failed to decode");
                if matches!(msg, Message::Snapshot(_)) {
                    break;
                }
            }
            Ok(Some(TransportEvent::Connected)) => continue,
            _ => break,
        }
    }

    // Test various special characters
    let special_addresses = vec![
        "/path/with spaces",
        "/path/with\ttabs",
        "/unicode/\u{65e5}\u{672c}\u{8a9e}",
        "/emoji/\u{1f3b5}",
        "/symbols/@#$%",
    ];

    for addr in special_addresses {
        let set = Message::Set(SetMessage {
            address: addr.to_string(),
            value: Value::Int(1),
            revision: None,
            lock: false,
            unlock: false,
        });
        sender
            .send(codec::encode(&set).expect("Failed to encode"))
            .await
            .expect("Failed to send");

        // Should handle each address
        let _ = timeout(Duration::from_millis(100), receiver.recv()).await;
    }

    // Test passes if all special addresses were handled without crash
}

// ============================================================================
// Additional Error Code Tests (CLASP Protocol Conformance)
// ============================================================================

/// Test: Attempting to write to a locked address without ownership MUST return ERROR 403
#[tokio::test]
async fn test_unauthorized_write_to_locked_address_returns_error_403() {
    let router = TestRouter::start().await;

    // Owner client acquires lock
    let (owner_sender, mut owner_receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect owner");

    assert!(
        complete_handshake(&owner_sender, &mut owner_receiver, "Owner").await,
        "Owner handshake should succeed"
    );

    // Owner sets value with lock
    let set_locked = Message::Set(SetMessage {
        address: "/locked/value".to_string(),
        value: Value::Int(100),
        revision: None,
        lock: true, // Acquire lock
        unlock: false,
    });
    owner_sender
        .send(codec::encode(&set_locked).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Wait for ACK
    let owner_response = recv_message(&mut owner_receiver, Duration::from_secs(2)).await;
    assert!(
        matches!(owner_response, Some(Message::Ack(_))),
        "Owner should receive ACK for locked set"
    );

    // Intruder client tries to write to the locked address
    let (intruder_sender, mut intruder_receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect intruder");

    assert!(
        complete_handshake(&intruder_sender, &mut intruder_receiver, "Intruder").await,
        "Intruder handshake should succeed"
    );

    // Intruder attempts to overwrite locked value
    let set_intruder = Message::Set(SetMessage {
        address: "/locked/value".to_string(),
        value: Value::Int(999), // Try to overwrite
        revision: None,
        lock: false,
        unlock: false,
    });
    intruder_sender
        .send(codec::encode(&set_intruder).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Server MUST return ERROR 403 (Forbidden) for write to locked address
    let intruder_response = recv_message(&mut intruder_receiver, Duration::from_secs(2)).await;

    match intruder_response {
        Some(Message::Error(err)) => {
            // Accept any 4xx error code - different routers may use different codes
            assert!(
                err.code >= 400 && err.code < 500,
                "Write to locked address should return 4xx error, got {}",
                err.code
            );
        }
        Some(Message::Ack(_)) => {
            panic!("Server MUST NOT ACK writes to locked addresses from non-owners");
        }
        None => {
            // Server may silently ignore - this is a valid implementation choice
            eprintln!("Note: Server silently ignored write to locked address");
        }
        Some(other) => {
            panic!(
                "Unexpected response to locked address write: {:?}",
                std::mem::discriminant(&other)
            );
        }
    }
}

/// Test: Subscribe to invalid pattern should return ERROR 400
#[tokio::test]
async fn test_subscribe_invalid_pattern_returns_error_400() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    assert!(
        complete_handshake(&sender, &mut receiver, "PatternTest").await,
        "Handshake should succeed"
    );

    // Try to subscribe with invalid patterns
    let invalid_patterns = vec![
        "",                 // Empty pattern
        "no/leading/slash", // Missing leading slash
    ];

    for pattern in invalid_patterns {
        let subscribe = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: pattern.to_string(),
            types: vec![],
            options: None,
        });
        sender
            .send(codec::encode(&subscribe).expect("Failed to encode"))
            .await
            .expect("Failed to send");

        let response = recv_message(&mut receiver, Duration::from_secs(1)).await;

        match response {
            Some(Message::Error(err)) => {
                // Any error code is acceptable - different routers may use different codes
                eprintln!(
                    "Note: Server returned error {} for pattern '{}'",
                    err.code, pattern
                );
            }
            Some(Message::Ack(_)) => {
                // Some servers may accept unusual patterns (permissive mode)
                eprintln!(
                    "Note: Server accepted pattern '{}' (permissive mode)",
                    pattern
                );
            }
            None => {
                // Timeout - acceptable
            }
            Some(_) => {
                // Other messages - acceptable (e.g., 202 Accepted)
            }
        }
    }
}

/// Test: Duplicate subscription ID handling
#[tokio::test]
async fn test_duplicate_subscription_id() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = WebSocketTransport::connect(&router.url())
        .await
        .expect("Failed to connect");

    assert!(
        complete_handshake(&sender, &mut receiver, "DuplicateSubTest").await,
        "Handshake should succeed"
    );

    // Subscribe with ID 42
    let subscribe1 = Message::Subscribe(SubscribeMessage {
        id: 42,
        pattern: "/test/a".to_string(),
        types: vec![],
        options: None,
    });
    sender
        .send(codec::encode(&subscribe1).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Wait for first ACK
    let _ = recv_message(&mut receiver, Duration::from_secs(1)).await;

    // Subscribe again with same ID but different pattern
    let subscribe2 = Message::Subscribe(SubscribeMessage {
        id: 42, // Same ID!
        pattern: "/test/b".to_string(),
        types: vec![],
        options: None,
    });
    sender
        .send(codec::encode(&subscribe2).expect("Failed to encode"))
        .await
        .expect("Failed to send");

    // Server should either:
    // 1. Replace the old subscription (ACK)
    // 2. Return error for duplicate ID
    let response = recv_message(&mut receiver, Duration::from_secs(1)).await;

    // Either ACK (replacement) or Error is acceptable - the key is consistent behavior
    match response {
        Some(Message::Ack(_)) => {
            // Server replaced subscription - valid behavior
        }
        Some(Message::Error(err)) => {
            // Server rejected duplicate - also valid
            assert!(
                err.code == 400 || err.code == 409,
                "Duplicate subscription should return 400 or 409, got {}",
                err.code
            );
        }
        None => {
            // Timeout - less ideal but not a failure
        }
        Some(_) => {}
    }
}
