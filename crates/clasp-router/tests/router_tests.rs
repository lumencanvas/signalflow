//! Router tests
//!
//! Tests for the CLASP router including:
//! - Transport-agnostic serving
//! - Session management
//! - Message routing
//! - Subscription handling

use clasp_core::{codec, HelloMessage, Message, SetMessage, SubscribeMessage, Value};
use clasp_router::{Router, RouterConfig};
use std::net::SocketAddr;
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
    use clasp_transport::{WebSocketServer, WebSocketTransport};
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
        let result = timeout(
            Duration::from_secs(2),
            WebSocketTransport::connect(&url),
        )
        .await;

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
            let result = timeout(
                Duration::from_secs(2),
                WebSocketTransport::connect(&url),
            )
            .await;
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
            timestamp: None,
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
        async fn complete_handshake<S: clasp_transport::TransportSender, R: clasp_transport::TransportReceiver>(
            sender: &S,
            receiver: &mut R,
            name: &str,
        ) {
            let hello = Message::Hello(HelloMessage {
                version: 2,
                name: name.to_string(),
                features: vec!["param".to_string()],
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
            types: None,
            options: None,
        });
        sender1.send(codec::encode(&subscribe).unwrap()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Client 2: Set value
        let set = Message::Set(SetMessage {
            address: "/test/sensor/temperature".to_string(),
            value: Value::Float(23.5),
            revision: None,
            timestamp: None,
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
