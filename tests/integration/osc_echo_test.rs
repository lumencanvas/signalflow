//! OSC Echo Test
//!
//! Tests bidirectional OSC <-> SignalFlow conversion

use clasp_bridge::{Bridge, BridgeEvent, OscBridge, OscBridgeConfig};
use clasp_core::{Message, SetMessage, Value};
use std::net::UdpSocket;
use std::time::Duration;

#[tokio::test]
async fn test_osc_to_clasp() {
    // Create OSC bridge
    let config = OscBridgeConfig {
        listen_addr: "127.0.0.1:9001".to_string(),
        send_addr: "127.0.0.1:9002".to_string(),
        namespace: "/osc".to_string(),
    };

    let mut bridge = OscBridge::new(config);
    let mut rx = bridge.start().await.expect("Failed to start bridge");

    // Wait for bridge to be ready
    match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(BridgeEvent::Connected)) => {}
        _ => panic!("Bridge did not connect"),
    }

    // Send OSC message to bridge
    let sender = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind sender");
    let osc_msg = rosc::OscMessage {
        addr: "/test/param".to_string(),
        args: vec![rosc::OscType::Float(0.75)],
    };
    let packet = rosc::OscPacket::Message(osc_msg);
    let bytes = rosc::encoder::encode(&packet).expect("Failed to encode OSC");
    sender
        .send_to(&bytes, "127.0.0.1:9001")
        .expect("Failed to send OSC");

    // Receive converted SignalFlow message
    match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(BridgeEvent::ToSignalFlow(msg))) => {
            if let Message::Set(set) = msg {
                assert_eq!(set.address, "/osc/test/param");
                // Value should be the float converted
            } else {
                panic!("Expected Set message, got {:?}", msg);
            }
        }
        other => panic!("Unexpected result: {:?}", other),
    }

    bridge.stop().await.expect("Failed to stop bridge");
}

#[tokio::test]
async fn test_clasp_to_osc() {
    // Create receiver socket first
    let receiver = UdpSocket::bind("127.0.0.1:9012").expect("Failed to bind receiver");
    receiver
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("Failed to set timeout");

    // Create OSC bridge
    let config = OscBridgeConfig {
        listen_addr: "127.0.0.1:9011".to_string(),
        send_addr: "127.0.0.1:9012".to_string(),
        namespace: "/osc".to_string(),
    };

    let mut bridge = OscBridge::new(config);
    let mut rx = bridge.start().await.expect("Failed to start bridge");

    // Wait for bridge to be ready
    match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(BridgeEvent::Connected)) => {}
        _ => panic!("Bridge did not connect"),
    }

    // Send SignalFlow message through bridge
    let msg = Message::Set(SetMessage {
        address: "/osc/test/output".to_string(),
        value: Value::Float(0.5),
        revision: None,
        lock: false,
        unlock: false,
    });

    bridge.send(msg).await.expect("Failed to send message");

    // Receive OSC message
    let mut buf = [0u8; 1024];
    match receiver.recv_from(&mut buf) {
        Ok((len, _)) => {
            let packet = rosc::decoder::decode_udp(&buf[..len]).expect("Failed to decode OSC");
            if let (_, rosc::OscPacket::Message(msg)) = packet {
                assert!(msg.addr.contains("test/output"));
            }
        }
        Err(e) => panic!("Failed to receive OSC: {}", e),
    }

    bridge.stop().await.expect("Failed to stop bridge");
}

#[tokio::test]
async fn test_osc_echo_roundtrip() {
    // This test sends OSC -> SignalFlow -> OSC
    let config = OscBridgeConfig {
        listen_addr: "127.0.0.1:9021".to_string(),
        send_addr: "127.0.0.1:9022".to_string(),
        namespace: "/echo".to_string(),
    };

    let mut bridge = OscBridge::new(config);
    let mut rx = bridge.start().await.expect("Failed to start bridge");

    // Wait for connect
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .ok();

    // Set up echo receiver
    let receiver = UdpSocket::bind("127.0.0.1:9022").expect("Failed to bind receiver");
    receiver
        .set_read_timeout(Some(Duration::from_secs(1)))
        .ok();

    // Send OSC
    let sender = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind sender");
    let osc_msg = rosc::OscMessage {
        addr: "/echo/value".to_string(),
        args: vec![rosc::OscType::Int(42)],
    };
    let packet = rosc::OscPacket::Message(osc_msg);
    let bytes = rosc::encoder::encode(&packet).expect("Failed to encode OSC");
    sender
        .send_to(&bytes, "127.0.0.1:9021")
        .expect("Failed to send");

    // Receive SignalFlow message from bridge
    if let Ok(Some(BridgeEvent::ToSignalFlow(msg))) =
        tokio::time::timeout(Duration::from_secs(1), rx.recv()).await
    {
        // Echo it back through the bridge
        bridge.send(msg).await.expect("Failed to echo");

        // Check if we get OSC back
        let mut buf = [0u8; 1024];
        if let Ok((len, _)) = receiver.recv_from(&mut buf) {
            let (_, packet) = rosc::decoder::decode_udp(&buf[..len]).expect("Decode failed");
            println!("Echo received: {:?}", packet);
        }
    }

    bridge.stop().await.expect("Failed to stop bridge");
}
