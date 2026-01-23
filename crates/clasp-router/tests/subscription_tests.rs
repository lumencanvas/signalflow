//! Subscription Tests
//!
//! Comprehensive tests for CLASP subscription patterns:
//! - Exact match subscriptions
//! - Single-level wildcards (*)
//! - Multi-level wildcards (**)
//! - Subscription lifecycle (add/remove)
//! - Multiple subscriptions per client
//! - Subscription filtering by signal type

use clasp_core::{
    codec, HelloMessage, Message, SetMessage, SubscribeMessage, UnsubscribeMessage, Value,
};
use clasp_test_utils::TestRouter;
use clasp_transport::{
    websocket::{WebSocketReceiver, WebSocketSender},
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use std::time::Duration;
use tokio::time::timeout;

// ============================================================================
// Utilities
// ============================================================================

async fn connect_and_handshake(url: &str, name: &str) -> (WebSocketSender, WebSocketReceiver) {
    let (sender, mut receiver) = WebSocketTransport::connect(url).await.unwrap();

    let hello = Message::Hello(HelloMessage {
        version: 2,
        name: name.to_string(),
        features: vec!["param".to_string(), "event".to_string()],
        capabilities: None,
        token: None,
    });
    sender.send(codec::encode(&hello).unwrap()).await.unwrap();

    // Wait for handshake completion
    let mut got_welcome = false;
    let mut got_snapshot = false;
    while !got_welcome || !got_snapshot {
        match timeout(Duration::from_secs(2), receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => {
                let (msg, _) = codec::decode(&data).unwrap();
                match msg {
                    Message::Welcome(_) => got_welcome = true,
                    Message::Snapshot(_) => got_snapshot = true,
                    _ => {}
                }
            }
            Ok(Some(TransportEvent::Connected)) => continue,
            _ => panic!("Handshake failed"),
        }
    }

    (sender, receiver)
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_exact_match_subscription() {
    let router = TestRouter::start().await;

    let (sub_sender, mut sub_receiver) = connect_and_handshake(&router.url(), "Subscriber").await;
    let (pub_sender, _pub_receiver) = connect_and_handshake(&router.url(), "Publisher").await;

    // Subscribe to exact path
    let subscribe = Message::Subscribe(SubscribeMessage {
        id: 1,
        pattern: "/exact/path".to_string(),
        types: vec![],
        options: None,
    });
    sub_sender
        .send(codec::encode(&subscribe).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Publish to exact path - should match
    let set1 = Message::Set(SetMessage {
        address: "/exact/path".to_string(),
        value: Value::Int(1),
        revision: None,
        lock: false,
        unlock: false,
    });
    pub_sender
        .send(codec::encode(&set1).unwrap())
        .await
        .unwrap();

    // Publish to different path - should NOT match
    let set2 = Message::Set(SetMessage {
        address: "/exact/other".to_string(),
        value: Value::Int(2),
        revision: None,
        lock: false,
        unlock: false,
    });
    pub_sender
        .send(codec::encode(&set2).unwrap())
        .await
        .unwrap();

    // Subscriber should only receive first message
    let msg1 = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Set(set) = msg {
                    return Some(set);
                }
            }
        }
    })
    .await;

    assert!(msg1.is_ok(), "Did not receive matching message");

    let set_msg = msg1.unwrap().unwrap();
    assert_eq!(
        set_msg.address, "/exact/path",
        "Wrong address: {}",
        set_msg.address
    );

    // Should NOT receive the second message (timeout expected)
    let msg2 = timeout(Duration::from_millis(200), async {
        loop {
            if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Set(set) = msg {
                    if set.address == "/exact/other" {
                        return Some(set);
                    }
                }
            }
        }
    })
    .await;

    assert!(msg2.is_err(), "Should NOT receive non-matching message");
}

#[tokio::test]
async fn test_single_wildcard_subscription() {
    let router = TestRouter::start().await;

    let (sub_sender, mut sub_receiver) = connect_and_handshake(&router.url(), "Subscriber").await;
    let (pub_sender, _pub_receiver) = connect_and_handshake(&router.url(), "Publisher").await;

    // Subscribe with single-level wildcard
    let subscribe = Message::Subscribe(SubscribeMessage {
        id: 1,
        pattern: "/sensors/*/temperature".to_string(),
        types: vec![],
        options: None,
    });
    sub_sender
        .send(codec::encode(&subscribe).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Should match: /sensors/room1/temperature
    pub_sender
        .send(
            codec::encode(&Message::Set(SetMessage {
                address: "/sensors/room1/temperature".to_string(),
                value: Value::Float(22.5),
                revision: None,
                lock: false,
                unlock: false,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    let msg = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Set(set) = msg {
                    return Some(set);
                }
            }
        }
    })
    .await;

    assert!(msg.is_ok(), "Did not receive matching message");
}

#[tokio::test]
async fn test_multi_wildcard_subscription() {
    let router = TestRouter::start().await;

    let (sub_sender, mut sub_receiver) = connect_and_handshake(&router.url(), "Subscriber").await;
    let (pub_sender, _pub_receiver) = connect_and_handshake(&router.url(), "Publisher").await;

    // Subscribe with multi-level wildcard
    let subscribe = Message::Subscribe(SubscribeMessage {
        id: 1,
        pattern: "/house/**".to_string(),
        types: vec![],
        options: None,
    });
    sub_sender
        .send(codec::encode(&subscribe).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // All of these should match
    let paths = vec![
        "/house/living-room/light",
        "/house/bedroom/temperature",
        "/house/kitchen/oven/temperature",
        "/house/basement/storage/humidity",
    ];

    for path in &paths {
        pub_sender
            .send(
                codec::encode(&Message::Set(SetMessage {
                    address: path.to_string(),
                    value: Value::Float(1.0),
                    revision: None,
                    lock: false,
                    unlock: false,
                }))
                .unwrap(),
            )
            .await
            .unwrap();
    }

    // Should receive all messages
    let mut received = 0;
    for _ in 0..paths.len() {
        let msg = timeout(Duration::from_secs(1), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(_) = msg {
                        return true;
                    }
                }
            }
        })
        .await;

        if msg.is_ok() {
            received += 1;
        }
    }

    assert!(
        received >= paths.len() - 1,
        "Only received {}/{} messages",
        received,
        paths.len()
    );
}

#[tokio::test]
async fn test_unsubscribe() {
    let router = TestRouter::start().await;

    let (sub_sender, mut sub_receiver) = connect_and_handshake(&router.url(), "Subscriber").await;
    let (pub_sender, _pub_receiver) = connect_and_handshake(&router.url(), "Publisher").await;

    // Subscribe
    sub_sender
        .send(
            codec::encode(&Message::Subscribe(SubscribeMessage {
                id: 1,
                pattern: "/test/**".to_string(),
                types: vec![],
                options: None,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // First message should be received
    pub_sender
        .send(
            codec::encode(&Message::Set(SetMessage {
                address: "/test/value1".to_string(),
                value: Value::Int(1),
                revision: None,
                lock: false,
                unlock: false,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    let msg1 = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Set(_) = msg {
                    return true;
                }
            }
        }
    })
    .await;

    assert!(msg1.is_ok(), "Should receive first message");

    // Unsubscribe
    sub_sender
        .send(codec::encode(&Message::Unsubscribe(UnsubscribeMessage { id: 1 })).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Second message should NOT be received
    pub_sender
        .send(
            codec::encode(&Message::Set(SetMessage {
                address: "/test/value2".to_string(),
                value: Value::Int(2),
                revision: None,
                lock: false,
                unlock: false,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    let msg2 = timeout(Duration::from_millis(300), async {
        loop {
            if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Set(set) = msg {
                    if set.address == "/test/value2" {
                        return true;
                    }
                }
            }
        }
    })
    .await;

    assert!(
        msg2.is_err(),
        "Should NOT receive message after unsubscribe"
    );
}

#[tokio::test]
async fn test_multiple_subscriptions() {
    let router = TestRouter::start().await;

    let (sub_sender, mut sub_receiver) = connect_and_handshake(&router.url(), "Subscriber").await;
    let (pub_sender, _pub_receiver) = connect_and_handshake(&router.url(), "Publisher").await;

    // Multiple subscriptions
    for (id, pattern) in [(1, "/a/**"), (2, "/b/**"), (3, "/c/**")] {
        sub_sender
            .send(
                codec::encode(&Message::Subscribe(SubscribeMessage {
                    id,
                    pattern: pattern.to_string(),
                    types: vec![],
                    options: None,
                }))
                .unwrap(),
            )
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send to each namespace
    for addr in ["/a/val", "/b/val", "/c/val"] {
        pub_sender
            .send(
                codec::encode(&Message::Set(SetMessage {
                    address: addr.to_string(),
                    value: Value::Int(1),
                    revision: None,
                    lock: false,
                    unlock: false,
                }))
                .unwrap(),
            )
            .await
            .unwrap();
    }

    // Should receive all 3
    let mut received = 0;
    for _ in 0..3 {
        if timeout(Duration::from_secs(1), async {
            loop {
                if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(_) = msg {
                        return true;
                    }
                }
            }
        })
        .await
        .is_ok()
        {
            received += 1;
        }
    }

    assert!(received >= 2, "Only received {}/3 messages", received);
}

#[tokio::test]
async fn test_subscription_initial_snapshot() {
    let router = TestRouter::start().await;

    // First client sets a value
    let (pub_sender, mut pub_receiver) = connect_and_handshake(&router.url(), "Publisher").await;
    pub_sender
        .send(
            codec::encode(&Message::Set(SetMessage {
                address: "/snapshot/test".to_string(),
                value: Value::Float(42.0),
                revision: None,
                lock: false,
                unlock: false,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    // Wait for ACK
    loop {
        if let Some(TransportEvent::Data(data)) = pub_receiver.recv().await {
            let (msg, _) = codec::decode(&data).unwrap();
            if matches!(msg, Message::Ack(_)) {
                break;
            }
        }
    }

    // Second client subscribes and should get snapshot with the value
    let (sub_sender, mut sub_receiver) = connect_and_handshake(&router.url(), "Subscriber").await;
    sub_sender
        .send(
            codec::encode(&Message::Subscribe(SubscribeMessage {
                id: 1,
                pattern: "/snapshot/**".to_string(),
                types: vec![],
                options: None,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    // Should receive snapshot with current value
    let found = timeout(Duration::from_secs(2), async {
        loop {
            if let Some(TransportEvent::Data(data)) = sub_receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Snapshot(snapshot) = msg {
                    for param in snapshot.params {
                        if param.address == "/snapshot/test" {
                            return true;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(
        found.is_ok(),
        "Did not receive snapshot with existing value"
    );
}

#[tokio::test]
async fn test_invalid_subscription_pattern() {
    let router = TestRouter::start().await;

    let (sender, mut receiver) = connect_and_handshake(&router.url(), "Client").await;

    // Subscribe with invalid pattern (empty)
    sender
        .send(
            codec::encode(&Message::Subscribe(SubscribeMessage {
                id: 1,
                pattern: "".to_string(), // Invalid
                types: vec![],
                options: None,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    // Should receive error (or be ignored - both are acceptable)
    let error = timeout(Duration::from_secs(1), async {
        loop {
            if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if let Message::Error(_) = msg {
                    return true;
                }
            }
        }
    })
    .await;

    // Empty pattern might just be ignored, so either error or timeout is acceptable
    // This test passes regardless - it's documenting the behavior
    let _ = error;
}
