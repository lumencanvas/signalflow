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
use tracing::info;

// ============================================================================
// Test Framework
// ============================================================================

struct TestResult {
    name: &'static str,
    passed: bool,
    message: String,
    duration_ms: u128,
}

impl TestResult {
    fn pass(name: &'static str, duration_ms: u128) -> Self {
        Self {
            name,
            passed: true,
            message: "OK".to_string(),
            duration_ms,
        }
    }

    fn fail(name: &'static str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name,
            passed: false,
            message: message.into(),
            duration_ms,
        }
    }
}

// ============================================================================
// Device Tests
// ============================================================================

fn test_device_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_creation";

    let device = Device::new("test-id-123".to_string(), "Test Device".to_string());

    if device.id == "test-id-123" && device.name == "Test Device" && device.endpoints.is_empty() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Device properties not set correctly", start.elapsed().as_millis())
    }
}

fn test_device_with_ws_endpoint() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_with_ws_endpoint";

    let device = Device::new("test-id".to_string(), "Test".to_string())
        .with_ws_endpoint("ws://192.168.1.100:7330/clasp");

    if let Some(ws_url) = device.ws_url() {
        if ws_url == "ws://192.168.1.100:7330/clasp" {
            return TestResult::pass(name, start.elapsed().as_millis());
        }
    }

    TestResult::fail(name, "WebSocket URL not set correctly", start.elapsed().as_millis())
}

fn test_device_with_udp_endpoint() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_with_udp_endpoint";

    let addr: SocketAddr = "192.168.1.100:7331".parse().unwrap();
    let device = Device::new("test-id".to_string(), "Test".to_string())
        .with_udp_endpoint(addr);

    if let Some(udp_addr) = device.udp_addr() {
        if udp_addr == addr {
            return TestResult::pass(name, start.elapsed().as_millis());
        }
    }

    TestResult::fail(name, "UDP endpoint not set correctly", start.elapsed().as_millis())
}

fn test_device_multiple_endpoints() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_multiple_endpoints";

    let addr: SocketAddr = "192.168.1.100:7331".parse().unwrap();
    let device = Device::new("test-id".to_string(), "Test".to_string())
        .with_ws_endpoint("ws://192.168.1.100:7330/clasp")
        .with_udp_endpoint(addr);

    let has_ws = device.ws_url().is_some();
    let has_udp = device.udp_addr().is_some();
    let endpoints_count = device.endpoints.len();

    if has_ws && has_udp && endpoints_count == 2 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, format!("Expected 2 endpoints, got {}", endpoints_count), start.elapsed().as_millis())
    }
}

fn test_device_touch() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_touch";

    let mut device = Device::new("test-id".to_string(), "Test".to_string());
    let initial = device.last_seen;

    // Wait a tiny bit
    std::thread::sleep(Duration::from_millis(10));

    device.touch();

    if device.last_seen > initial {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "last_seen not updated by touch()", start.elapsed().as_millis())
    }
}

fn test_device_staleness() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_staleness";

    let device = Device::new("test-id".to_string(), "Test".to_string());

    // Should not be stale with 10s timeout
    let not_stale = !device.is_stale(Duration::from_secs(10));

    // Should be stale with 0ms timeout
    let stale = device.is_stale(Duration::from_millis(0));

    if not_stale && stale {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Staleness check incorrect", start.elapsed().as_millis())
    }
}

// ============================================================================
// DeviceInfo Tests
// ============================================================================

fn test_device_info_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_info_default";

    let info = DeviceInfo::default();

    let has_features = !info.features.is_empty();
    let is_not_bridge = !info.bridge;
    let version_set = info.version == clasp_core::PROTOCOL_VERSION;

    if has_features && is_not_bridge && version_set {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Default DeviceInfo not correct", start.elapsed().as_millis())
    }
}

fn test_device_info_with_features() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_info_with_features";

    let info = DeviceInfo::default()
        .with_features(vec!["param".to_string(), "stream".to_string(), "gesture".to_string()]);

    if info.features.len() == 3 && info.features.contains(&"gesture".to_string()) {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Features not set correctly", start.elapsed().as_millis())
    }
}

fn test_device_info_as_bridge() -> TestResult {
    let start = std::time::Instant::now();
    let name = "device_info_as_bridge";

    let info = DeviceInfo::default().as_bridge("osc");

    if info.bridge && info.bridge_protocol == Some("osc".to_string()) {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Bridge configuration incorrect", start.elapsed().as_millis())
    }
}

// ============================================================================
// Discovery Tests
// ============================================================================

fn test_discovery_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_creation";

    let discovery = Discovery::new();

    if discovery.devices().count() == 0 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "New discovery should have no devices", start.elapsed().as_millis())
    }
}

fn test_discovery_with_config() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_with_config";

    let config = DiscoveryConfig {
        mdns: false,
        broadcast: true,
        broadcast_port: 7331,
        timeout: Duration::from_secs(10),
    };

    let _discovery = Discovery::with_config(config.clone());

    // Just verify it doesn't panic
    TestResult::pass(name, start.elapsed().as_millis())
}

fn test_discovery_config_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_config_default";

    let config = DiscoveryConfig::default();

    if config.mdns && config.broadcast && config.broadcast_port == clasp_core::DEFAULT_DISCOVERY_PORT {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Default config not correct", start.elapsed().as_millis())
    }
}

fn test_discovery_manual_add() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_manual_add";

    let mut discovery = Discovery::new();

    let device = Device::new("manual-1".to_string(), "Manual Device".to_string())
        .with_ws_endpoint("ws://localhost:7330");

    discovery.add(device);

    if discovery.devices().count() == 1 && discovery.get("manual-1").is_some() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Manual device add failed", start.elapsed().as_millis())
    }
}

fn test_discovery_manual_remove() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_manual_remove";

    let mut discovery = Discovery::new();

    let device = Device::new("removable".to_string(), "Removable".to_string());
    discovery.add(device);

    let removed = discovery.remove("removable");

    if removed.is_some() && discovery.devices().count() == 0 && discovery.get("removable").is_none() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Device removal failed", start.elapsed().as_millis())
    }
}

fn test_discovery_get_nonexistent() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_get_nonexistent";

    let discovery = Discovery::new();

    if discovery.get("nonexistent").is_none() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Should return None for nonexistent device", start.elapsed().as_millis())
    }
}

fn test_discovery_multiple_devices() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_multiple_devices";

    let mut discovery = Discovery::new();

    for i in 0..5 {
        let device = Device::new(format!("device-{}", i), format!("Device {}", i));
        discovery.add(device);
    }

    if discovery.devices().count() == 5 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(name, "Multiple devices not added correctly", start.elapsed().as_millis())
    }
}

fn test_discovery_overwrite_device() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_overwrite_device";

    let mut discovery = Discovery::new();

    let device1 = Device::new("same-id".to_string(), "First".to_string());
    let device2 = Device::new("same-id".to_string(), "Second".to_string());

    discovery.add(device1);
    discovery.add(device2);

    if discovery.devices().count() == 1 {
        if let Some(device) = discovery.get("same-id") {
            if device.name == "Second" {
                return TestResult::pass(name, start.elapsed().as_millis());
            }
        }
    }

    TestResult::fail(name, "Device overwrite not working correctly", start.elapsed().as_millis())
}

// ============================================================================
// DiscoveryEvent Tests
// ============================================================================

fn test_discovery_event_found() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_event_found";

    let device = Device::new("found-1".to_string(), "Found Device".to_string());
    let event = DiscoveryEvent::Found(device.clone());

    match event {
        DiscoveryEvent::Found(d) => {
            if d.id == "found-1" {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Found event device ID mismatch", start.elapsed().as_millis())
            }
        }
        _ => TestResult::fail(name, "Wrong event variant", start.elapsed().as_millis()),
    }
}

fn test_discovery_event_lost() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_event_lost";

    let event = DiscoveryEvent::Lost("lost-device-id".to_string());

    match event {
        DiscoveryEvent::Lost(id) => {
            if id == "lost-device-id" {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Lost event ID mismatch", start.elapsed().as_millis())
            }
        }
        _ => TestResult::fail(name, "Wrong event variant", start.elapsed().as_millis()),
    }
}

fn test_discovery_event_error() -> TestResult {
    let start = std::time::Instant::now();
    let name = "discovery_event_error";

    let event = DiscoveryEvent::Error("Network error".to_string());

    match event {
        DiscoveryEvent::Error(msg) => {
            if msg == "Network error" {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Error event message mismatch", start.elapsed().as_millis())
            }
        }
        _ => TestResult::fail(name, "Wrong event variant", start.elapsed().as_millis()),
    }
}

// ============================================================================
// Network Tests (require actual network access)
// Note: These tests may fail in restricted environments
// ============================================================================

#[cfg(feature = "broadcast")]
async fn test_broadcast_responder_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "broadcast_responder_creation";

    use clasp_discovery::broadcast::BroadcastResponder;

    // Try to bind on a random port
    match BroadcastResponder::bind(
        0, // Let OS choose port
        "Test Responder".to_string(),
        vec!["param".to_string(), "event".to_string()],
    )
    .await
    {
        Ok(_responder) => TestResult::pass(name, start.elapsed().as_millis()),
        Err(e) => TestResult::fail(name, format!("Failed to create responder: {}", e), start.elapsed().as_millis()),
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Discovery Tests                               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut tests = vec![
        // Device tests
        test_device_creation(),
        test_device_with_ws_endpoint(),
        test_device_with_udp_endpoint(),
        test_device_multiple_endpoints(),
        test_device_touch(),
        test_device_staleness(),

        // DeviceInfo tests
        test_device_info_default(),
        test_device_info_with_features(),
        test_device_info_as_bridge(),

        // Discovery tests
        test_discovery_creation(),
        test_discovery_with_config(),
        test_discovery_config_default(),
        test_discovery_manual_add(),
        test_discovery_manual_remove(),
        test_discovery_get_nonexistent(),
        test_discovery_multiple_devices(),
        test_discovery_overwrite_device(),

        // DiscoveryEvent tests
        test_discovery_event_found(),
        test_discovery_event_lost(),
        test_discovery_event_error(),
    ];

    // Add network tests if broadcast feature is enabled
    #[cfg(feature = "broadcast")]
    {
        tests.push(test_broadcast_responder_creation().await);
    }

    let mut passed = 0;
    let mut failed = 0;

    println!("┌──────────────────────────────────────┬────────┬──────────┐");
    println!("│ Test                                 │ Status │ Time     │");
    println!("├──────────────────────────────────────┼────────┼──────────┤");

    for test in &tests {
        let status = if test.passed { "✓ PASS" } else { "✗ FAIL" };
        let color = if test.passed { "\x1b[32m" } else { "\x1b[31m" };
        println!(
            "│ {:<36} │ {}{:<6}\x1b[0m │ {:>6}ms │",
            test.name, color, status, test.duration_ms
        );

        if test.passed {
            passed += 1;
        } else {
            failed += 1;
            println!("│   └─ {:<56} │", &test.message[..test.message.len().min(56)]);
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
