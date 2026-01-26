//! Router tests
//!
//! Tests for the CLASP router including:
//! - Transport-agnostic serving
//! - Session management
//! - Message routing
//! - Subscription handling

use clasp_core::{codec, HelloMessage, Message, SecurityMode, SetMessage, SubscribeMessage, Value};
use clasp_router::{Router, RouterConfig};
use std::time::Duration;
use tokio::time::timeout;

/// Test router creation with default config
#[tokio::test]
async fn test_router_creation() {
    let router = Router::default();
    assert_eq!(router.session_count(), 0);
    assert_eq!(router.subscription_count(), 0);
}

/// Test router creation with custom config
#[tokio::test]
async fn test_router_custom_config() {
    let config = RouterConfig {
        name: "Test Router".to_string(),
        max_sessions: 50,
        session_timeout: 120,
        features: vec!["param".to_string(), "event".to_string()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 1000,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 0,
        max_messages_per_second: 0,
        rate_limiting_enabled: false,
        ..Default::default()
    };
    let router = Router::new(config);
    assert_eq!(router.session_count(), 0);
}

/// Test router stop functionality
#[tokio::test]
async fn test_router_stop() {
    let router = Router::default();
    router.stop();
    // Router should be stoppable even when not running
}

/// Test router state access
#[tokio::test]
async fn test_router_state_access() {
    let router = Router::default();
    let state = router.state();
    // State should be empty initially
    assert!(state.get_state("/test").is_none());
}

#[cfg(feature = "websocket")]
mod websocket_tests {
    use super::*;
    use clasp_transport::{Transport, TransportReceiver, TransportSender, WebSocketTransport};
    use tokio::net::TcpListener;

    /// Find an available port for testing
    async fn find_available_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    }

    /// Test WebSocket server binding
    #[tokio::test]
    async fn test_websocket_server_bind() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        // Start router in background
        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify we can connect
        let url = format!("ws://{}", addr);
        let result = timeout(Duration::from_secs(2), WebSocketTransport::connect(&url)).await;

        // Connection should succeed
        assert!(result.is_ok(), "Should connect to WebSocket server");

        router_handle.abort();
    }

    /// Test multiple clients connecting
    #[tokio::test]
    async fn test_multiple_websocket_clients() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{}", addr);

        // Connect multiple clients
        let mut connections = vec![];
        for _ in 0..3 {
            let result = timeout(Duration::from_secs(2), WebSocketTransport::connect(&url)).await;
            assert!(result.is_ok());
            connections.push(result.unwrap().unwrap());
        }

        // All connections should be active
        assert_eq!(connections.len(), 3);

        router_handle.abort();
    }

    /// Test HELLO/WELCOME handshake
    #[tokio::test]
    async fn test_hello_welcome_handshake() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{}", addr);
        let (sender, mut receiver) = WebSocketTransport::connect(&url).await.unwrap();

        // Send HELLO
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: "Test Client".to_string(),
            features: vec!["param".to_string()],
            capabilities: None,
            token: None,
        });
        let hello_bytes = codec::encode(&hello).unwrap();
        sender.send(hello_bytes).await.unwrap();

        // Wait for WELCOME
        use clasp_transport::TransportEvent;
        let response = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(event) = receiver.recv().await {
                    match event {
                        TransportEvent::Data(data) => {
                            return Some(data);
                        }
                        TransportEvent::Connected => continue,
                        _ => return None,
                    }
                }
            }
        })
        .await;

        assert!(response.is_ok(), "Should receive WELCOME response");
        let data = response.unwrap().unwrap();
        let (msg, _) = codec::decode(&data).unwrap();

        match msg {
            Message::Welcome(_) => (), // Expected
            other => panic!("Expected Welcome, got {:?}", other),
        }

        router_handle.abort();
    }

    /// Test SET message routing
    #[tokio::test]
    async fn test_set_message_routing() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{}", addr);
        let (sender, mut receiver) = WebSocketTransport::connect(&url).await.unwrap();

        // Send HELLO first
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: "Test Client".to_string(),
            features: vec!["param".to_string()],
            capabilities: None,
            token: None,
        });
        sender.send(codec::encode(&hello).unwrap()).await.unwrap();

        // Wait for WELCOME
        use clasp_transport::TransportEvent;
        loop {
            if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if matches!(msg, Message::Welcome(_)) {
                    break;
                }
            }
        }

        // Skip SNAPSHOT
        loop {
            if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                if matches!(msg, Message::Snapshot(_)) {
                    break;
                }
            }
        }

        // Now send SET
        let set = Message::Set(SetMessage {
            address: "/test/value".to_string(),
            value: Value::Float(42.0),
            revision: None,
            lock: false,
            unlock: false,
        });
        sender.send(codec::encode(&set).unwrap()).await.unwrap();

        // Wait for ACK
        let ack_result = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if matches!(msg, Message::Ack(_)) {
                        return Some(msg);
                    }
                }
            }
        })
        .await;

        assert!(ack_result.is_ok(), "Should receive ACK for SET");

        router_handle.abort();
    }

    /// Test subscription and message delivery
    #[tokio::test]
    async fn test_subscription_message_delivery() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{}", addr);

        // Client 1: Subscriber
        let (sender1, mut receiver1) = WebSocketTransport::connect(&url).await.unwrap();

        // Client 2: Publisher
        let (sender2, mut receiver2) = WebSocketTransport::connect(&url).await.unwrap();

        use clasp_transport::TransportEvent;

        // Helper to complete handshake
        async fn complete_handshake<
            S: clasp_transport::TransportSender,
            R: clasp_transport::TransportReceiver,
        >(
            sender: &S,
            receiver: &mut R,
            name: &str,
        ) {
            let hello = Message::Hello(HelloMessage {
                version: 2,
                name: name.to_string(),
                features: vec!["param".to_string()],
                capabilities: None,
                token: None,
            });
            sender.send(codec::encode(&hello).unwrap()).await.unwrap();

            // Wait for WELCOME and SNAPSHOT
            let mut got_welcome = false;
            let mut got_snapshot = false;
            while !got_welcome || !got_snapshot {
                if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    match msg {
                        Message::Welcome(_) => got_welcome = true,
                        Message::Snapshot(_) => got_snapshot = true,
                        _ => {}
                    }
                }
            }
        }

        // Complete handshakes
        complete_handshake(&sender1, &mut receiver1, "Subscriber").await;
        complete_handshake(&sender2, &mut receiver2, "Publisher").await;

        // Client 1: Subscribe to /test/**
        let subscribe = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: "/test/**".to_string(),
            types: vec![],
            options: None,
        });
        sender1
            .send(codec::encode(&subscribe).unwrap())
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Client 2: Set value
        let set = Message::Set(SetMessage {
            address: "/test/sensor/temperature".to_string(),
            value: Value::Float(23.5),
            revision: None,
            lock: false,
            unlock: false,
        });
        sender2.send(codec::encode(&set).unwrap()).await.unwrap();

        // Client 1 should receive the SET
        let received = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(TransportEvent::Data(data)) = receiver1.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Set(set_msg) = msg {
                        if set_msg.address == "/test/sensor/temperature" {
                            return Some(set_msg);
                        }
                    }
                }
            }
        })
        .await;

        assert!(received.is_ok(), "Subscriber should receive SET message");
        let set_msg = received.unwrap().unwrap();
        assert_eq!(set_msg.value, Value::Float(23.5));

        router_handle.abort();
    }
}

/// Tests for generic serve_on method
mod serve_on_tests {
    use super::*;

    /// Test that router can be created and stopped without serving
    #[tokio::test]
    async fn test_router_lifecycle() {
        let router = Router::default();
        assert_eq!(router.session_count(), 0);
        router.stop();
    }
}

/// Tests for router state management
mod state_tests {
    use super::*;

    /// Test state persistence across messages
    #[tokio::test]
    async fn test_state_empty_initially() {
        let router = Router::default();
        let state = router.state();
        assert!(state.get_state("/any/path").is_none());
    }
}

/// Tests for P2P signaling
#[cfg(feature = "websocket")]
mod p2p_tests {
    use super::*;
    use clasp_core::{signal_address, PublishMessage, SignalType, P2P_SIGNAL_PREFIX};
    use clasp_transport::{
        Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
    };
    use tokio::net::TcpListener;

    async fn find_available_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    }

    async fn complete_handshake<S: TransportSender, R: TransportReceiver>(
        sender: &S,
        receiver: &mut R,
        name: &str,
    ) -> String {
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: name.to_string(),
            features: vec!["param".to_string()],
            capabilities: None,
            token: None,
        });
        sender.send(codec::encode(&hello).unwrap()).await.unwrap();

        let mut session_id = String::new();
        let mut got_welcome = false;
        let mut got_snapshot = false;

        while !got_welcome || !got_snapshot {
            if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                let (msg, _) = codec::decode(&data).unwrap();
                match msg {
                    Message::Welcome(w) => {
                        session_id = w.session.clone();
                        got_welcome = true;
                    }
                    Message::Snapshot(_) => got_snapshot = true,
                    _ => {}
                }
            }
        }

        session_id
    }

    /// Test P2P signal routing between two clients
    #[tokio::test]
    async fn test_p2p_signal_routing() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{}", addr);

        // Connect two clients
        let (sender_a, mut receiver_a) = WebSocketTransport::connect(&url).await.unwrap();
        let (sender_b, mut receiver_b) = WebSocketTransport::connect(&url).await.unwrap();

        // Complete handshakes and get session IDs
        let session_a = complete_handshake(&sender_a, &mut receiver_a, "Client A").await;
        let session_b = complete_handshake(&sender_b, &mut receiver_b, "Client B").await;

        // Client B subscribes to its P2P signal address
        let subscribe = Message::Subscribe(SubscribeMessage {
            id: 1,
            pattern: format!("{}{}", P2P_SIGNAL_PREFIX, session_b),
            types: vec![],
            options: None,
        });
        sender_b
            .send(codec::encode(&subscribe).unwrap())
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Client A sends a P2P signal to Client B
        let signal_payload = serde_json::json!({
            "type": "offer",
            "from": session_a,
            "sdp": "v=0\r\n...",
            "correlation_id": "test-123"
        });

        let signal_value = json_to_value(signal_payload);
        let publish = Message::Publish(PublishMessage {
            address: signal_address(&session_b),
            signal: Some(SignalType::Event),
            value: None,
            payload: Some(signal_value),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
            timeline: None,
        });

        sender_a
            .send(codec::encode(&publish).unwrap())
            .await
            .unwrap();

        // Client B should receive the signal
        let received = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(TransportEvent::Data(data)) = receiver_b.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Publish(pub_msg) = msg {
                        if pub_msg.address.starts_with(P2P_SIGNAL_PREFIX) {
                            return Some(pub_msg);
                        }
                    }
                }
            }
        })
        .await;

        assert!(received.is_ok(), "Client B should receive P2P signal");
        let pub_msg = received.unwrap().unwrap();
        assert_eq!(pub_msg.address, signal_address(&session_b));

        router_handle.abort();
    }

    /// Test P2P signal to non-existent session returns error
    #[tokio::test]
    async fn test_p2p_signal_to_nonexistent_session() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::default();

        let router_handle = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let _ = router.serve_websocket(&addr).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{}", addr);
        let (sender, mut receiver) = WebSocketTransport::connect(&url).await.unwrap();
        let session_id = complete_handshake(&sender, &mut receiver, "Client").await;

        // Send signal to non-existent session
        let signal_payload = serde_json::json!({
            "type": "offer",
            "from": session_id,
            "sdp": "v=0\r\n...",
            "correlation_id": "test-123"
        });

        let publish = Message::Publish(PublishMessage {
            address: signal_address("nonexistent-session-id"),
            signal: Some(SignalType::Event),
            value: None,
            payload: Some(json_to_value(signal_payload)),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
            timeline: None,
        });

        sender.send(codec::encode(&publish).unwrap()).await.unwrap();

        // Should receive error
        let error = timeout(Duration::from_secs(2), async {
            loop {
                if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                    let (msg, _) = codec::decode(&data).unwrap();
                    if let Message::Error(err) = msg {
                        return Some(err);
                    }
                }
            }
        })
        .await;

        assert!(
            error.is_ok(),
            "Should receive error for nonexistent session"
        );
        let err = error.unwrap().unwrap();
        assert_eq!(err.code, 404);

        router_handle.abort();
    }

    fn json_to_value(json: serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(json_to_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let map: std::collections::HashMap<String, Value> = obj
                    .into_iter()
                    .map(|(k, v)| (k, json_to_value(v)))
                    .collect();
                Value::Map(map)
            }
        }
    }
}
