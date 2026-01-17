//! Bridge Tests (clasp-bridge)
//!
//! Tests for protocol bridges including:
//! - MQTT Bridge configuration and message conversion
//! - HTTP Bridge configuration, value conversion, and server mode
//! - WebSocket Bridge configuration and message parsing
//! - SocketIO configuration
//!
//! Note: Full integration tests require external services (MQTT broker, etc.)

use clasp_bridge::{Bridge, BridgeEvent};
use clasp_core::Value;
use std::collections::HashMap;
use std::time::Duration;

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
// MQTT Bridge Tests
// ============================================================================

fn test_mqtt_config_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "mqtt_config_default";

    use clasp_bridge::mqtt::MqttBridgeConfig;

    let config = MqttBridgeConfig::default();

    if config.broker_host == "localhost"
        && config.broker_port == 1883
        && config.qos == 0
        && config.keep_alive_secs == 60
        && config.namespace == "/mqtt"
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Default MQTT config incorrect",
            start.elapsed().as_millis(),
        )
    }
}

fn test_mqtt_config_custom() -> TestResult {
    let start = std::time::Instant::now();
    let name = "mqtt_config_custom";

    use clasp_bridge::mqtt::MqttBridgeConfig;

    let config = MqttBridgeConfig {
        broker_host: "mqtt.example.com".to_string(),
        broker_port: 8883,
        client_id: "test-client".to_string(),
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
        subscribe_topics: vec!["sensors/#".to_string()],
        qos: 1,
        keep_alive_secs: 30,
        namespace: "/sensors".to_string(),
    };

    if config.broker_host == "mqtt.example.com"
        && config.broker_port == 8883
        && config.qos == 1
        && config.username == Some("user".to_string())
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Custom MQTT config incorrect",
            start.elapsed().as_millis(),
        )
    }
}

fn test_mqtt_bridge_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "mqtt_bridge_creation";

    use clasp_bridge::mqtt::{MqttBridge, MqttBridgeConfig};

    let config = MqttBridgeConfig::default();
    let bridge = MqttBridge::new(config);

    let bridge_config = bridge.config();

    if bridge_config.protocol == "mqtt" && bridge_config.bidirectional && !bridge.is_running() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "MQTT bridge not created correctly",
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// HTTP Bridge Tests
// ============================================================================

fn test_http_config_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "http_config_default";

    use clasp_bridge::http::{HttpBridgeConfig, HttpMode};

    let config = HttpBridgeConfig::default();

    if config.mode == HttpMode::Server
        && config.cors_enabled
        && config.base_path == "/api"
        && config.timeout_secs == 30
        && config.namespace == "/http"
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Default HTTP config incorrect",
            start.elapsed().as_millis(),
        )
    }
}

fn test_http_config_client_mode() -> TestResult {
    let start = std::time::Instant::now();
    let name = "http_config_client_mode";

    use clasp_bridge::http::{HttpBridgeConfig, HttpMode};

    let config = HttpBridgeConfig {
        mode: HttpMode::Client,
        url: "https://api.example.com".to_string(),
        endpoints: vec![],
        cors_enabled: false,
        cors_origins: vec![],
        base_path: "/v1".to_string(),
        timeout_secs: 60,
        namespace: "/api".to_string(),
    };

    if config.mode == HttpMode::Client
        && config.url == "https://api.example.com"
        && config.timeout_secs == 60
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "HTTP client config incorrect",
            start.elapsed().as_millis(),
        )
    }
}

fn test_http_bridge_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "http_bridge_creation";

    use clasp_bridge::http::{HttpBridge, HttpBridgeConfig};

    let config = HttpBridgeConfig::default();
    let bridge = HttpBridge::new(config);

    let bridge_config = bridge.config();

    if bridge_config.protocol == "http" && bridge_config.bidirectional && !bridge.is_running() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "HTTP bridge not created correctly",
            start.elapsed().as_millis(),
        )
    }
}

async fn test_http_server_start_stop() -> TestResult {
    let start = std::time::Instant::now();
    let name = "http_server_start_stop";

    use clasp_bridge::http::{HttpBridge, HttpBridgeConfig, HttpMode};

    // Find available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = HttpBridgeConfig {
        mode: HttpMode::Server,
        url: format!("127.0.0.1:{}", port),
        ..Default::default()
    };

    let mut bridge = HttpBridge::new(config);

    // Start the bridge
    match bridge.start().await {
        Ok(mut rx) => {
            // Wait for connected event
            let event = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
            let connected = matches!(event, Ok(Some(BridgeEvent::Connected)));

            // Stop the bridge
            let _ = bridge.stop().await;

            if connected && !bridge.is_running() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    "HTTP server didn't connect/stop properly",
                    start.elapsed().as_millis(),
                )
            }
        }
        Err(e) => TestResult::fail(
            name,
            format!("Failed to start: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

// ============================================================================
// WebSocket Bridge Tests
// ============================================================================

fn test_websocket_config_default() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_config_default";

    use clasp_bridge::websocket::{WebSocketBridgeConfig, WsMessageFormat, WsMode};

    let config = WebSocketBridgeConfig::default();

    if config.mode == WsMode::Client
        && config.format == WsMessageFormat::Json
        && config.auto_reconnect
        && config.ping_interval_secs == 30
        && config.namespace == "/ws"
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Default WebSocket config incorrect",
            start.elapsed().as_millis(),
        )
    }
}

fn test_websocket_config_server_mode() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_config_server_mode";

    use clasp_bridge::websocket::{WebSocketBridgeConfig, WsMessageFormat, WsMode};

    let config = WebSocketBridgeConfig {
        mode: WsMode::Server,
        url: "0.0.0.0:8080".to_string(),
        path: Some("/ws".to_string()),
        format: WsMessageFormat::MsgPack,
        ping_interval_secs: 60,
        auto_reconnect: false,
        reconnect_delay_secs: 10,
        headers: HashMap::new(),
        namespace: "/live".to_string(),
    };

    if config.mode == WsMode::Server
        && config.format == WsMessageFormat::MsgPack
        && config.path == Some("/ws".to_string())
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "WebSocket server config incorrect",
            start.elapsed().as_millis(),
        )
    }
}

fn test_websocket_bridge_creation() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_bridge_creation";

    use clasp_bridge::websocket::{WebSocketBridge, WebSocketBridgeConfig};

    let config = WebSocketBridgeConfig::default();
    let bridge = WebSocketBridge::new(config);

    let bridge_config = bridge.config();

    if bridge_config.protocol == "websocket" && bridge_config.bidirectional && !bridge.is_running()
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "WebSocket bridge not created correctly",
            start.elapsed().as_millis(),
        )
    }
}

async fn test_websocket_server_start_stop() -> TestResult {
    let start = std::time::Instant::now();
    let name = "websocket_server_start_stop";

    use clasp_bridge::websocket::{WebSocketBridge, WebSocketBridgeConfig, WsMode};

    // Find available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = WebSocketBridgeConfig {
        mode: WsMode::Server,
        url: format!("127.0.0.1:{}", port),
        ..Default::default()
    };

    let mut bridge = WebSocketBridge::new(config);

    // Start the bridge
    match bridge.start().await {
        Ok(mut rx) => {
            // Wait for connected event
            let event = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
            let connected = matches!(event, Ok(Some(BridgeEvent::Connected)));

            // Stop the bridge
            let _ = bridge.stop().await;

            tokio::time::sleep(Duration::from_millis(100)).await;

            if connected && !bridge.is_running() {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    "WebSocket server didn't connect/stop properly",
                    start.elapsed().as_millis(),
                )
            }
        }
        Err(e) => TestResult::fail(
            name,
            format!("Failed to start: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

// ============================================================================
// Transform Tests (using the Transform enum directly)
// ============================================================================

fn test_transform_scale() -> TestResult {
    let start = std::time::Instant::now();
    let name = "transform_scale";

    use clasp_bridge::transform::{Transform, TransformState};

    let transform = Transform::Scale {
        from_min: 0.0,
        from_max: 10.0,
        to_min: 0.0,
        to_max: 100.0,
    };

    let mut state = TransformState::default();

    // Test scale: 5.0 from [0,10] to [0,100] = 50.0
    let result = transform.apply(&Value::Float(5.0), &mut state);

    match result {
        Value::Float(f) => {
            if (f - 50.0).abs() < 0.001 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Scale wrong: {} != 50.0", f),
                    start.elapsed().as_millis(),
                )
            }
        }
        _ => TestResult::fail(
            name,
            "Transform did not produce float",
            start.elapsed().as_millis(),
        ),
    }
}

fn test_transform_clamp() -> TestResult {
    let start = std::time::Instant::now();
    let name = "transform_clamp";

    use clasp_bridge::transform::{Transform, TransformState};

    let transform = Transform::Clamp {
        min: 0.0,
        max: 100.0,
    };

    let mut state = TransformState::default();

    // Test clamping - value above max
    let result = transform.apply(&Value::Float(150.0), &mut state);

    match result {
        Value::Float(f) => {
            if (f - 100.0).abs() < 0.001 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Clamp wrong: {} != 100.0", f),
                    start.elapsed().as_millis(),
                )
            }
        }
        _ => TestResult::fail(
            name,
            "Transform did not produce float",
            start.elapsed().as_millis(),
        ),
    }
}

fn test_transform_invert() -> TestResult {
    let start = std::time::Instant::now();
    let name = "transform_invert";

    use clasp_bridge::transform::{Transform, TransformState};

    let transform = Transform::Invert;

    let mut state = TransformState::default();

    // Test invert: 0.3 -> 0.7
    let result = transform.apply(&Value::Float(0.3), &mut state);

    match result {
        Value::Float(f) => {
            if (f - 0.7).abs() < 0.001 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Invert wrong: {} != 0.7", f),
                    start.elapsed().as_millis(),
                )
            }
        }
        _ => TestResult::fail(
            name,
            "Transform did not produce float",
            start.elapsed().as_millis(),
        ),
    }
}

fn test_transform_expression() -> TestResult {
    let start = std::time::Instant::now();
    let name = "transform_expression";

    use clasp_bridge::transform::{Transform, TransformState};

    let transform = Transform::Expression {
        expr: "value * 2 + 10".to_string(),
    };

    let mut state = TransformState::default();

    // Test expression: 5.0 * 2 + 10 = 20.0
    let result = transform.apply(&Value::Float(5.0), &mut state);

    match result {
        Value::Float(f) => {
            if (f - 20.0).abs() < 0.001 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Expression wrong: {} != 20.0", f),
                    start.elapsed().as_millis(),
                )
            }
        }
        _ => TestResult::fail(
            name,
            "Transform did not produce float",
            start.elapsed().as_millis(),
        ),
    }
}

// ============================================================================
// Mapping Tests
// ============================================================================

fn test_mapping_simple() -> TestResult {
    let start = std::time::Instant::now();
    let name = "mapping_simple";

    use clasp_bridge::mapping::AddressMapping;

    let mapping = AddressMapping::new("/osc/synth/cutoff", "/synth/cutoff");

    // Test exact match
    let result = mapping.map_address("/osc/synth/cutoff");
    if result == Some("/synth/cutoff".to_string()) {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Mapping wrong: {:?}", result),
            start.elapsed().as_millis(),
        )
    }
}

fn test_mapping_table() -> TestResult {
    let start = std::time::Instant::now();
    let name = "mapping_table";

    use clasp_bridge::mapping::{AddressMapping, MappingTable};

    let mut table = MappingTable::new();
    table.add(AddressMapping::new("/osc/fader/1", "/clasp/fader/1"));
    table.add(AddressMapping::new("/osc/fader/2", "/clasp/fader/2"));

    // Test table lookup
    let result1 = table.map("/osc/fader/1");
    let result2 = table.map("/osc/fader/2");
    let result_none = table.map("/osc/fader/3");

    if result1 == Some("/clasp/fader/1".to_string())
        && result2 == Some("/clasp/fader/2".to_string())
        && result_none.is_none()
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Mapping table lookup failed",
            start.elapsed().as_millis(),
        )
    }
}

fn test_mapping_with_transform() -> TestResult {
    let start = std::time::Instant::now();
    let name = "mapping_with_transform";

    use clasp_bridge::mapping::{AddressMapping, MappingTable, ValueTransform};

    let mut table = MappingTable::new();
    let mapping = AddressMapping::new("/midi/cc", "/clasp/cc")
        .with_transform(ValueTransform::scale(0.0, 127.0, 0.0, 1.0));
    table.add(mapping);

    // Test transform
    let value = Value::Float(63.5);
    let result = table.transform("/midi/cc", &value);

    match result {
        Value::Float(f) => {
            // 63.5 / 127 = 0.5
            if (f - 0.5).abs() < 0.01 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Transform wrong: {} != 0.5", f),
                    start.elapsed().as_millis(),
                )
            }
        }
        _ => TestResult::fail(
            name,
            "Transform did not produce float",
            start.elapsed().as_millis(),
        ),
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Bridge Tests                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        // MQTT Bridge tests
        test_mqtt_config_default(),
        test_mqtt_config_custom(),
        test_mqtt_bridge_creation(),
        // HTTP Bridge tests
        test_http_config_default(),
        test_http_config_client_mode(),
        test_http_bridge_creation(),
        test_http_server_start_stop().await,
        // WebSocket Bridge tests
        test_websocket_config_default(),
        test_websocket_config_server_mode(),
        test_websocket_bridge_creation(),
        test_websocket_server_start_stop().await,
        // Transform tests
        test_transform_scale(),
        test_transform_clamp(),
        test_transform_invert(),
        test_transform_expression(),
        // Mapping tests
        test_mapping_simple(),
        test_mapping_table(),
        test_mapping_with_transform(),
    ];

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
            println!(
                "│   └─ {:<56} │",
                &test.message[..test.message.len().min(56)]
            );
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
