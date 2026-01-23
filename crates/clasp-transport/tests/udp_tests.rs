//! UDP Transport Tests (clasp-transport)
//!
//! Tests for the UDP transport implementation including:
//! - Binding and local address
//! - Send/receive operations
//! - Broadcast functionality
//! - Multiple concurrent sockets

use bytes::Bytes;
use clasp_transport::udp::{UdpBroadcast, UdpConfig, UdpTransport};
use clasp_transport::{TransportEvent, TransportSender};
use std::collections::HashSet;
use std::time::Duration;

// ============================================================================
// Basic Binding Tests
// ============================================================================

#[tokio::test]
async fn test_udp_bind_default() {
    let transport = UdpTransport::bind("127.0.0.1:0")
        .await
        .expect("Bind should succeed");

    let addr = transport.local_addr().expect("Should get local address");

    assert!(addr.port() > 0, "Port should be > 0");
}

#[tokio::test]
async fn test_udp_bind_with_config() {
    let config = UdpConfig {
        recv_buffer_size: 32768,
        max_packet_size: 1500,
    };

    let transport = UdpTransport::bind_with_config("127.0.0.1:0", config)
        .await
        .expect("Bind with config should succeed");

    assert!(
        transport.local_addr().is_ok(),
        "Should be able to get local address"
    );
}

#[tokio::test]
async fn test_udp_bind_specific_port() {
    // Find an available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let transport = UdpTransport::bind(&format!("127.0.0.1:{}", port))
        .await
        .expect("Bind to specific port should succeed");

    let addr = transport.local_addr().expect("Should get local address");

    assert_eq!(addr.port(), port, "Should bind to the specified port");
}

// ============================================================================
// Send/Receive Tests
// ============================================================================

#[tokio::test]
async fn test_udp_send_receive() {
    let server = UdpTransport::bind("127.0.0.1:0")
        .await
        .expect("Server bind should succeed");

    let client = UdpTransport::bind("127.0.0.1:0")
        .await
        .expect("Client bind should succeed");

    let server_addr = server.local_addr().unwrap();
    let client_addr = client.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Send from client
    client
        .send_to(b"hello udp", server_addr)
        .await
        .expect("Send should succeed");

    // Receive with timeout
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;

    match result {
        Ok(Some((TransportEvent::Data(data), from))) => {
            assert_eq!(data.as_ref(), b"hello udp", "Data should match");
            assert_eq!(
                from.port(),
                client_addr.port(),
                "Source port should match client"
            );
        }
        Ok(Some((TransportEvent::Error(e), _))) => {
            panic!("Receive error: {}", e);
        }
        Ok(None) => {
            panic!("Receiver closed unexpectedly");
        }
        Err(_) => {
            panic!("Timeout waiting for data");
        }
        _ => {
            panic!("Unexpected event");
        }
    }
}

#[tokio::test]
async fn test_udp_sender_to() {
    let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Create a sender targeting the server
    let sender = client.sender_to(server_addr);

    // Verify sender is connected
    assert!(sender.is_connected(), "Sender should be connected");

    // Send using TransportSender trait
    sender
        .send(Bytes::from_static(b"via sender"))
        .await
        .expect("Send should succeed");

    // Receive
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;

    match result {
        Ok(Some((TransportEvent::Data(data), _))) => {
            assert_eq!(data.as_ref(), b"via sender", "Data should match");
        }
        _ => {
            panic!("Failed to receive data");
        }
    }
}

#[tokio::test]
async fn test_udp_multiple_messages() {
    let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Send multiple messages
    for i in 0..10 {
        let msg = format!("message {}", i);
        client.send_to(msg.as_bytes(), server_addr).await.unwrap();
    }

    // Receive all messages
    let mut received = 0;
    for _ in 0..10 {
        let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;
        if let Ok(Some((TransportEvent::Data(_), _))) = result {
            received += 1;
        }
    }

    // Allow some packet loss (UDP is unreliable)
    assert!(
        received >= 8,
        "Should receive at least 8/10 messages, got {}",
        received
    );
}

#[tokio::test]
async fn test_udp_large_packet() {
    let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let server_addr = server.local_addr().unwrap();
    let mut receiver = server.start_receiver();

    // Send a reasonably large packet (8KB - safe for all platforms)
    let large_data = vec![0xAB; 8192];
    client.send_to(&large_data, server_addr).await.unwrap();

    // Receive
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from()).await;

    match result {
        Ok(Some((TransportEvent::Data(data), _))) => {
            assert_eq!(data.len(), 8192, "Data length should be 8192");
            assert_eq!(data[0], 0xAB, "Data content should match");
        }
        _ => {
            panic!("Failed to receive large packet");
        }
    }
}

// ============================================================================
// Broadcast Tests
// ============================================================================

#[tokio::test]
async fn test_udp_set_broadcast() {
    let transport = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    transport
        .set_broadcast(true)
        .expect("Should be able to enable broadcast");
}

#[tokio::test]
async fn test_udp_broadcast_creation() {
    let _broadcast = UdpBroadcast::new(7331)
        .await
        .expect("Broadcast creation should succeed");
}

// ============================================================================
// Concurrent Tests
// ============================================================================

#[tokio::test]
async fn test_udp_concurrent_sockets() {
    // Create multiple sockets
    let mut sockets = vec![];
    for _ in 0..5 {
        let socket = UdpTransport::bind("127.0.0.1:0")
            .await
            .expect("Bind should succeed");
        sockets.push(socket);
    }

    // Verify all have unique ports
    let ports: Vec<u16> = sockets
        .iter()
        .map(|s| s.local_addr().unwrap().port())
        .collect();
    let unique_ports: HashSet<u16> = ports.iter().cloned().collect();

    assert_eq!(unique_ports.len(), 5, "All ports should be unique");
}

#[tokio::test]
async fn test_udp_bidirectional() {
    let socket_a = UdpTransport::bind("127.0.0.1:0").await.unwrap();
    let socket_b = UdpTransport::bind("127.0.0.1:0").await.unwrap();

    let addr_b = socket_b.local_addr().unwrap();

    let mut recv_a = socket_a.start_receiver();
    let mut recv_b = socket_b.start_receiver();

    // A sends to B
    socket_a.send_to(b"hello from A", addr_b).await.unwrap();

    // B receives and sends back
    let b_received = tokio::time::timeout(Duration::from_secs(2), recv_b.recv_from()).await;

    match b_received {
        Ok(Some((TransportEvent::Data(data), from))) => {
            assert_eq!(data.as_ref(), b"hello from A", "B should receive from A");
            socket_b.send_to(b"hello from B", from).await.unwrap();
        }
        _ => {
            panic!("B didn't receive from A");
        }
    }

    // A receives response
    let a_received = tokio::time::timeout(Duration::from_secs(2), recv_a.recv_from()).await;

    match a_received {
        Ok(Some((TransportEvent::Data(data), _))) => {
            assert_eq!(data.as_ref(), b"hello from B", "A should receive from B");
        }
        _ => {
            panic!("A didn't receive from B");
        }
    }
}
