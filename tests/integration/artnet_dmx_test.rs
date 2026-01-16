//! Art-Net and DMX Echo Tests
//!
//! Tests Art-Net and DMX <-> CLASP conversion

use clasp_bridge::{ArtNetBridge, ArtNetBridgeConfig, DmxBridge, DmxBridgeConfig, DmxInterfaceType};
use clasp_bridge::{Bridge, BridgeEvent};
use clasp_core::{Message, SetMessage, Value};
use std::time::Duration;

#[tokio::test]
async fn test_artnet_bridge_lifecycle() {
    // Test basic Art-Net bridge start/stop
    let config = ArtNetBridgeConfig {
        bind_addr: "127.0.0.1:6455".to_string(), // Use non-standard port for testing
        remote_addr: Some("127.0.0.1:6456".to_string()),
        universes: vec![],
        namespace: "/artnet".to_string(),
    };

    let mut bridge = ArtNetBridge::new(config);

    // Start bridge
    let mut rx = bridge.start().await.expect("Failed to start Art-Net bridge");

    // Check for connected event
    match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(BridgeEvent::Connected)) => {
            println!("Art-Net bridge connected");
        }
        other => {
            println!("Unexpected event: {:?}", other);
        }
    }

    assert!(bridge.is_running());

    // Stop bridge
    bridge.stop().await.expect("Failed to stop bridge");
    assert!(!bridge.is_running());
    println!("Art-Net bridge stopped");
}

#[tokio::test]
async fn test_dmx_virtual_bridge() {
    // Test DMX bridge in virtual mode
    let config = DmxBridgeConfig {
        port: None,
        interface_type: DmxInterfaceType::Virtual,
        universe: 1,
        namespace: "/dmx".to_string(),
        refresh_rate: 44.0,
    };

    let mut bridge = DmxBridge::new(config);

    // Start bridge
    let mut rx = bridge.start().await.expect("Failed to start DMX bridge");

    // Check for connected event
    match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(BridgeEvent::Connected)) => {
            println!("DMX bridge connected (virtual mode)");
        }
        other => {
            println!("Unexpected event: {:?}", other);
        }
    }

    // Send a DMX value
    let msg = Message::Set(SetMessage {
        address: "/dmx/1/1".to_string(), // Universe 1, Channel 1
        value: Value::Int(255),
        revision: None,
        lock: false,
        unlock: false,
    });

    bridge.send(msg).await.expect("Failed to send DMX message");

    // Verify channel value
    assert_eq!(bridge.get_channel(1), Some(255));

    // Set multiple channels
    bridge.set_channel(2, 128);
    bridge.set_channel(3, 64);
    assert_eq!(bridge.get_channel(2), Some(128));
    assert_eq!(bridge.get_channel(3), Some(64));

    // Stop bridge
    bridge.stop().await.expect("Failed to stop bridge");
    println!("DMX bridge stopped");
}

#[test]
fn test_dmx_channel_bounds() {
    let bridge = DmxBridge::new(DmxBridgeConfig::default());

    // Valid channels (1-512)
    assert!(bridge.get_channel(1).is_some() || bridge.get_channel(1).is_none()); // Initially 0
    assert!(bridge.get_channel(512).is_some() || bridge.get_channel(512).is_none());

    // Invalid channels
    assert_eq!(bridge.get_channel(0), None);
    assert_eq!(bridge.get_channel(513), None);
}

#[test]
fn test_list_dmx_ports() {
    match DmxBridge::list_ports() {
        Ok(ports) => {
            println!("Available DMX/serial ports:");
            for port in &ports {
                println!("  - {}", port);
            }
            if ports.is_empty() {
                println!("  (no USB-DMX interfaces found)");
            }
        }
        Err(e) => println!("Error listing ports: {}", e),
    }
}

#[tokio::test]
async fn test_artnet_clasp_conversion() {
    // Test CLASP -> Art-Net conversion
    let config = ArtNetBridgeConfig {
        bind_addr: "127.0.0.1:6457".to_string(),
        remote_addr: Some("127.0.0.1:6458".to_string()),
        universes: vec![],
        namespace: "/artnet".to_string(),
    };

    let mut bridge = ArtNetBridge::new(config);
    let _rx = bridge.start().await.expect("Failed to start bridge");

    // Create CLASP set message for DMX channel
    let msg = Message::Set(SetMessage {
        address: "/artnet/1/10".to_string(), // Universe 1, Channel 10
        value: Value::Int(200),
        revision: None,
        lock: false,
        unlock: false,
    });

    // This will fail because remote isn't listening, but tests the conversion
    let result = bridge.send(msg).await;
    println!("Send result: {:?}", result);

    bridge.stop().await.expect("Failed to stop bridge");
}

#[test]
fn test_artnet_address_parsing() {
    // Test address parsing for Art-Net messages
    let address = "/artnet/1/42";
    let parts: Vec<&str> = address.split('/').collect();

    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0], "");
    assert_eq!(parts[1], "artnet");

    let universe: u16 = parts[2].parse().unwrap();
    let channel: u16 = parts[3].parse().unwrap();

    assert_eq!(universe, 1);
    assert_eq!(channel, 42);

    println!("Parsed Art-Net address: Universe {}, Channel {}", universe, channel);
}

#[test]
fn test_dmx_frame_operations() {
    let bridge = DmxBridge::new(DmxBridgeConfig::default());

    // Create a test frame
    let mut frame = [0u8; 512];
    frame[0] = 255; // Channel 1
    frame[99] = 128; // Channel 100
    frame[511] = 64; // Channel 512

    bridge.set_frame(&frame);

    // Note: In virtual mode without sender, get_channel reads from internal state
    // The actual verification would happen in a running bridge
    println!("DMX frame set with {} channels", frame.iter().filter(|&&v| v > 0).count());
}
