//! QUIC Transport Tests (clasp-transport)
//!
//! Tests for the QUIC transport implementation including:
//! - Configuration
//! - Client/Server creation
//! - Connection establishment
//! - Stream operations
//! - Datagram support
//!
//! Note: These tests require the 'quic' feature to be enabled

#![cfg(feature = "quic")]

use std::net::SocketAddr;
use std::time::Duration;

use bytes::Bytes;
use clasp_transport::quic::{CertVerification, QuicConfig, QuicTransport, CLASP_ALPN};
use clasp_transport::{TransportReceiver, TransportSender};
use rcgen::{generate_simple_self_signed, CertifiedKey};

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_self_signed_cert() -> (Vec<u8>, Vec<u8>) {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let CertifiedKey { cert, key_pair } =
        generate_simple_self_signed(subject_alt_names).expect("Cert generation failed");

    (cert.der().to_vec(), key_pair.serialize_der())
}

async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_quic_config_default() {
    let config = QuicConfig::default();

    assert!(config.enable_0rtt, "enable_0rtt should be true by default");
    assert_eq!(config.keep_alive_ms, 5000, "keep_alive_ms should be 5000");
    assert_eq!(
        config.idle_timeout_ms, 30000,
        "idle_timeout_ms should be 30000"
    );
    assert_eq!(config.initial_window, 10, "initial_window should be 10");
}

#[tokio::test]
async fn test_quic_config_custom() {
    let config = QuicConfig {
        enable_0rtt: false,
        keep_alive_ms: 10000,
        idle_timeout_ms: 60000,
        initial_window: 20,
        cert_verification: CertVerification::SkipVerification,
    };

    assert!(!config.enable_0rtt, "enable_0rtt should be false");
    assert_eq!(config.keep_alive_ms, 10000, "keep_alive_ms should be 10000");
    assert_eq!(
        config.idle_timeout_ms, 60000,
        "idle_timeout_ms should be 60000"
    );
    assert_eq!(config.initial_window, 20, "initial_window should be 20");
}

// ============================================================================
// ALPN Test
// ============================================================================

#[tokio::test]
async fn test_quic_alpn_protocol() {
    assert_eq!(CLASP_ALPN, b"clasp/2", "ALPN should be 'clasp/2'");
}

// ============================================================================
// Client Creation Tests
// ============================================================================

#[tokio::test]
async fn test_quic_client_creation() {
    let result = QuicTransport::new_client();
    assert!(result.is_ok(), "Client creation should succeed");
}

#[tokio::test]
async fn test_quic_client_with_config() {
    let config = QuicConfig {
        enable_0rtt: true,
        keep_alive_ms: 3000,
        idle_timeout_ms: 15000,
        initial_window: 5,
        cert_verification: CertVerification::SkipVerification,
    };

    let result = QuicTransport::new_client_with_config(config);
    assert!(
        result.is_ok(),
        "Client creation with config should succeed: {:?}",
        result.err()
    );
}

// ============================================================================
// Server Creation Tests
// ============================================================================

#[tokio::test]
async fn test_quic_server_creation() {
    let port = find_available_port().await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let (cert, key) = generate_self_signed_cert();

    let server = QuicTransport::new_server(addr, cert, key);
    assert!(
        server.is_ok(),
        "Server creation should succeed: {:?}",
        server.err()
    );

    let server = server.unwrap();
    let local_addr = server.local_addr();
    assert!(
        local_addr.is_ok(),
        "Should be able to get local address: {:?}",
        local_addr.err()
    );

    let local = local_addr.unwrap();
    assert_eq!(
        local.port(),
        port,
        "Server should be listening on the correct port"
    );
}

// ============================================================================
// Connection Tests
// ============================================================================

#[tokio::test]
async fn test_quic_client_server_connect() {
    let port = find_available_port().await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let (cert, key) = generate_self_signed_cert();

    // Create server
    let server =
        QuicTransport::new_server(addr, cert, key).expect("Server creation should succeed");

    // Create client
    let client = QuicTransport::new_client().expect("Client creation should succeed");

    // Spawn server accept task
    let server_handle = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), server.accept())
            .await
            .expect("Accept should not timeout")
            .expect("Accept should succeed")
    });

    // Give server time to start listening
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect from client
    let client_conn = client
        .connect(addr, "localhost")
        .await
        .expect("Client connect should succeed");

    // Wait for server accept
    let server_conn = server_handle.await.expect("Server task should not panic");

    // Verify both connections are established
    let client_remote = client_conn.remote_address();
    let _server_remote = server_conn.remote_address();

    assert_eq!(
        client_remote.port(),
        port,
        "Client should be connected to the correct port"
    );
}

// ============================================================================
// Stream Tests
// ============================================================================

#[tokio::test]
async fn test_quic_bidirectional_stream() {
    let port = find_available_port().await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let (cert, key) = generate_self_signed_cert();

    let server =
        QuicTransport::new_server(addr, cert, key).expect("Server creation should succeed");

    let client = QuicTransport::new_client().expect("Client creation should succeed");

    // Server task - echoes back received data
    let server_handle = tokio::spawn(async move {
        let conn = server.accept().await.expect("Accept should succeed");
        let (sender, mut receiver) = conn.accept_bi().await.expect("Accept bi should succeed");

        // Read message from client
        match receiver.recv().await {
            Some(clasp_transport::TransportEvent::Data(data)) => {
                // Echo back
                sender.send(data).await.expect("Send should succeed");
            }
            other => panic!("Unexpected event: {:?}", other),
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client connects and sends
    let conn = client
        .connect(addr, "localhost")
        .await
        .expect("Client connect should succeed");

    let (sender, mut receiver) = conn.open_bi().await.expect("Open bi should succeed");

    // Send test message
    let test_data = Bytes::from_static(b"Hello QUIC!");
    sender
        .send(test_data.clone())
        .await
        .expect("Send should succeed");

    // Wait for echo
    let received = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .expect("Should not timeout waiting for echo");

    match received {
        Some(clasp_transport::TransportEvent::Data(data)) => {
            assert_eq!(data, test_data, "Echoed data should match sent data");
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // Clean up server
    let _ = server_handle.await;
}
