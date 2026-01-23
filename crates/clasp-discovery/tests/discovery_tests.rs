//! Discovery Tests (clasp-discovery)
//!
//! Tests for the CLASP device discovery system including:
//! - Device struct creation and management
//! - DeviceInfo configuration
//! - Discovery struct operations
//! - UDP broadcast discovery
//! - Note: mDNS tests require network access and are marked as such

use clasp_discovery::{Device, DeviceInfo, Discovery, DiscoveryConfig, DiscoveryEvent};
use std::net::SocketAddr;
use std::time::Duration;

// ============================================================================
// Device Tests
// ============================================================================

#[tokio::test]
async fn test_device_creation() {
    let device = Device::new("test-id-123".to_string(), "Test Device".to_string());

    assert_eq!(device.id, "test-id-123");
    assert_eq!(device.name, "Test Device");
    assert!(device.endpoints.is_empty());
}

#[tokio::test]
async fn test_device_with_ws_endpoint() {
    let device = Device::new("test-id".to_string(), "Test".to_string())
        .with_ws_endpoint("ws://192.168.1.100:7330/clasp");

    let ws_url = device.ws_url();
    assert!(ws_url.is_some(), "WebSocket URL should be set");
    assert_eq!(ws_url.unwrap(), "ws://192.168.1.100:7330/clasp");
}

#[tokio::test]
async fn test_device_with_udp_endpoint() {
    let addr: SocketAddr = "192.168.1.100:7331".parse().unwrap();
    let device = Device::new("test-id".to_string(), "Test".to_string()).with_udp_endpoint(addr);

    let udp_addr = device.udp_addr();
    assert!(udp_addr.is_some(), "UDP endpoint should be set");
    assert_eq!(udp_addr.unwrap(), addr);
}

#[tokio::test]
async fn test_device_multiple_endpoints() {
    let addr: SocketAddr = "192.168.1.100:7331".parse().unwrap();
    let device = Device::new("test-id".to_string(), "Test".to_string())
        .with_ws_endpoint("ws://192.168.1.100:7330/clasp")
        .with_udp_endpoint(addr);

    assert!(device.ws_url().is_some(), "Should have WebSocket endpoint");
    assert!(device.udp_addr().is_some(), "Should have UDP endpoint");
    assert_eq!(device.endpoints.len(), 2, "Should have 2 endpoints");
}

#[tokio::test]
async fn test_device_touch() {
    let mut device = Device::new("test-id".to_string(), "Test".to_string());
    let initial = device.last_seen;

    // Wait a tiny bit
    std::thread::sleep(Duration::from_millis(10));

    device.touch();

    assert!(
        device.last_seen > initial,
        "last_seen should be updated by touch()"
    );
}

#[tokio::test]
async fn test_device_staleness() {
    let device = Device::new("test-id".to_string(), "Test".to_string());

    // Should not be stale with 10s timeout
    assert!(
        !device.is_stale(Duration::from_secs(10)),
        "Device should not be stale with 10s timeout"
    );

    // Should be stale with 0ms timeout
    assert!(
        device.is_stale(Duration::from_millis(0)),
        "Device should be stale with 0ms timeout"
    );
}

// ============================================================================
// DeviceInfo Tests
// ============================================================================

#[tokio::test]
async fn test_device_info_default() {
    let info = DeviceInfo::default();

    assert!(
        !info.features.is_empty(),
        "Default DeviceInfo should have features"
    );
    assert!(!info.bridge, "Default DeviceInfo should not be a bridge");
    assert_eq!(
        info.version,
        clasp_core::PROTOCOL_VERSION,
        "Version should match protocol version"
    );
}

#[tokio::test]
async fn test_device_info_with_features() {
    let info = DeviceInfo::default().with_features(vec![
        "param".to_string(),
        "stream".to_string(),
        "gesture".to_string(),
    ]);

    assert_eq!(info.features.len(), 3, "Should have 3 features");
    assert!(
        info.features.contains(&"gesture".to_string()),
        "Should contain 'gesture' feature"
    );
}

#[tokio::test]
async fn test_device_info_as_bridge() {
    let info = DeviceInfo::default().as_bridge("osc");

    assert!(info.bridge, "Should be marked as bridge");
    assert_eq!(
        info.bridge_protocol,
        Some("osc".to_string()),
        "Bridge protocol should be 'osc'"
    );
}

// ============================================================================
// Discovery Tests
// ============================================================================

#[tokio::test]
async fn test_discovery_creation() {
    let discovery = Discovery::new();

    assert_eq!(
        discovery.devices().count(),
        0,
        "New discovery should have no devices"
    );
}

#[tokio::test]
async fn test_discovery_with_config() {
    let config = DiscoveryConfig {
        mdns: false,
        broadcast: true,
        broadcast_port: 7331,
        timeout: Duration::from_secs(10),
    };

    let _discovery = Discovery::with_config(config);
    // Just verify it doesn't panic
}

#[tokio::test]
async fn test_discovery_config_default() {
    let config = DiscoveryConfig::default();

    assert!(config.mdns, "Default config should have mDNS enabled");
    assert!(
        config.broadcast,
        "Default config should have broadcast enabled"
    );
    assert_eq!(
        config.broadcast_port,
        clasp_core::DEFAULT_DISCOVERY_PORT,
        "Default broadcast port should match"
    );
}

#[tokio::test]
async fn test_discovery_manual_add() {
    let mut discovery = Discovery::new();

    let device = Device::new("manual-1".to_string(), "Manual Device".to_string())
        .with_ws_endpoint("ws://localhost:7330");

    discovery.add(device);

    assert_eq!(discovery.devices().count(), 1, "Should have 1 device");
    assert!(
        discovery.get("manual-1").is_some(),
        "Should be able to get device by ID"
    );
}

#[tokio::test]
async fn test_discovery_manual_remove() {
    let mut discovery = Discovery::new();

    let device = Device::new("removable".to_string(), "Removable".to_string());
    discovery.add(device);

    let removed = discovery.remove("removable");

    assert!(removed.is_some(), "Remove should return the device");
    assert_eq!(
        discovery.devices().count(),
        0,
        "Should have no devices after removal"
    );
    assert!(
        discovery.get("removable").is_none(),
        "Should not find removed device"
    );
}

#[tokio::test]
async fn test_discovery_get_nonexistent() {
    let discovery = Discovery::new();

    assert!(
        discovery.get("nonexistent").is_none(),
        "Should return None for nonexistent device"
    );
}

#[tokio::test]
async fn test_discovery_multiple_devices() {
    let mut discovery = Discovery::new();

    for i in 0..5 {
        let device = Device::new(format!("device-{}", i), format!("Device {}", i));
        discovery.add(device);
    }

    assert_eq!(discovery.devices().count(), 5, "Should have 5 devices");
}

#[tokio::test]
async fn test_discovery_overwrite_device() {
    let mut discovery = Discovery::new();

    let device1 = Device::new("same-id".to_string(), "First".to_string());
    let device2 = Device::new("same-id".to_string(), "Second".to_string());

    discovery.add(device1);
    discovery.add(device2);

    assert_eq!(
        discovery.devices().count(),
        1,
        "Should have only 1 device (overwritten)"
    );

    let device = discovery.get("same-id").expect("Device should exist");
    assert_eq!(
        device.name, "Second",
        "Device should be the second one added"
    );
}

// ============================================================================
// DiscoveryEvent Tests
// ============================================================================

#[tokio::test]
async fn test_discovery_event_found() {
    let device = Device::new("found-1".to_string(), "Found Device".to_string());
    let event = DiscoveryEvent::Found(device);

    match event {
        DiscoveryEvent::Found(d) => {
            assert_eq!(d.id, "found-1", "Found event should contain correct device");
        }
        _ => panic!("Expected Found event variant"),
    }
}

#[tokio::test]
async fn test_discovery_event_lost() {
    let event = DiscoveryEvent::Lost("lost-device-id".to_string());

    match event {
        DiscoveryEvent::Lost(id) => {
            assert_eq!(id, "lost-device-id", "Lost event should contain correct ID");
        }
        _ => panic!("Expected Lost event variant"),
    }
}

#[tokio::test]
async fn test_discovery_event_error() {
    let event = DiscoveryEvent::Error("Network error".to_string());

    match event {
        DiscoveryEvent::Error(msg) => {
            assert_eq!(
                msg, "Network error",
                "Error event should contain correct message"
            );
        }
        _ => panic!("Expected Error event variant"),
    }
}

// ============================================================================
// Network Tests (require actual network access)
// Note: These tests may fail in restricted environments
// ============================================================================

#[cfg(feature = "broadcast")]
#[tokio::test]
async fn test_broadcast_responder_creation() {
    use clasp_discovery::broadcast::BroadcastResponder;

    // Try to bind on a random port (let OS choose)
    let result = BroadcastResponder::bind(
        0, // Let OS choose port
        "Test Responder".to_string(),
        vec!["param".to_string(), "event".to_string()],
    )
    .await;

    assert!(
        result.is_ok(),
        "Should be able to create broadcast responder: {:?}",
        result.err()
    );
}
