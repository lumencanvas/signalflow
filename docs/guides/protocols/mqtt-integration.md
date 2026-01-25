# MQTT Integration Guide

This guide explains how to integrate MQTT (Message Queuing Telemetry Transport) with CLASP.

## Two Integration Modes

CLASP supports MQTT in two modes:

1. **MQTT Server Adapter** (new in 3.1.0): The CLASP router accepts MQTT clients directly, no external broker needed
2. **MQTT Bridge**: Connect to an external MQTT broker and bridge messages bidirectionally

## MQTT Server Adapter (Recommended)

The MQTT server adapter lets the CLASP router accept MQTT clients directly on port 1883. This eliminates the need for an external broker.

### Setup

Enable the `mqtt-server` feature:

```toml
[dependencies]
clasp-router = { version = "3.1", features = ["mqtt-server"] }
```

### Configuration

```rust
use clasp_router::{Router, RouterConfig, MultiProtocolConfig, MqttServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Router::new(RouterConfig::default());

    let config = MultiProtocolConfig {
        websocket_addr: Some("0.0.0.0:7330".into()),
        mqtt: Some(MqttServerConfig {
            bind_addr: "0.0.0.0:1883".into(),
            namespace: "/mqtt".into(),
            require_auth: false,
            max_clients: 100,
            session_timeout_secs: 300,
            ..Default::default()
        }),
        ..Default::default()
    };

    router.serve_all(config).await?;
    Ok(())
}
```

### Using the Relay CLI

```bash
# Start relay with MQTT support
clasp-relay --mqtt-port 1883

# Custom namespace
clasp-relay --mqtt-port 1883 --mqtt-namespace /iot
```

### Connecting MQTT Clients

Any standard MQTT client can connect:

```bash
# Subscribe
mosquitto_sub -h localhost -p 1883 -t "sensors/#"

# Publish
mosquitto_pub -h localhost -p 1883 -t "sensors/temp" -m "25.5"
```

### Topic to Address Mapping

MQTT topics are prefixed with the configured namespace:

| MQTT Topic | CLASP Address |
|------------|---------------|
| `sensors/temp` | `/mqtt/sensors/temp` |
| `home/living/light` | `/mqtt/home/living/light` |

MQTT wildcards are converted to CLASP patterns:

| MQTT Pattern | CLASP Pattern |
|--------------|---------------|
| `sensors/#` | `/mqtt/sensors/**` |
| `sensors/+/temp` | `/mqtt/sensors/*/temp` |

---

## MQTT Bridge (External Broker)

The MQTT bridge connects to an external MQTT broker and translates messages bidirectionally:

- **MQTT to CLASP**: MQTT messages published to subscribed topics are translated to CLASP SET messages
- **CLASP to MQTT**: CLASP SET/PUBLISH messages are translated to MQTT publish operations

## Basic Setup

### Starting the MQTT Bridge

The MQTT bridge connects to an MQTT broker and translates messages bidirectionally:

```rust
use clasp_bridge::mqtt::{MqttBridge, MqttBridgeConfig};

let config = MqttBridgeConfig {
    broker_host: "localhost".to_string(),
    broker_port: 1883,
    client_id: "clasp-bridge".to_string(),
    subscribe_topics: vec!["sensors/#".to_string(), "actuators/+".to_string()],
    qos: 0,
    namespace: "/mqtt".to_string(),
    ..Default::default()
};

let mut bridge = MqttBridge::new(config);
let mut rx = bridge.start().await?;

// Handle bridge events (forward to router in real deployment)
while let Some(event) = rx.recv().await {
    match event {
        BridgeEvent::ToClasp(msg) => {
            // Forward to CLASP router
        }
        BridgeEvent::FromClasp(msg) => {
            // Publish to MQTT broker
        }
        _ => {}
    }
}
```

### Using the CLI

```bash
# Start MQTT bridge connection
clasp mqtt --host localhost --port 1883

# Subscribe to specific topics
clasp mqtt --host broker.example.com --topic "sensors/#" --topic "home/+"

# With authentication
clasp mqtt --host broker.example.com --username user --password pass
```

## Topic → Address Mapping

MQTT topics are mapped to CLASP addresses using the namespace prefix:

| MQTT Topic | CLASP Address | Direction |
|------------|---------------|-----------|
| `sensors/temp` | `/mqtt/sensors/temp` | MQTT → CLASP |
| `actuators/light/1` | `/mqtt/actuators/light/1` | MQTT → CLASP |
| `/mqtt/output/value` | `output/value` | CLASP → MQTT |

**Default namespace**: `/mqtt`

The namespace can be configured in `MqttBridgeConfig`:

```rust
let config = MqttBridgeConfig {
    namespace: "/iot".to_string(),  // Custom namespace
    ..Default::default()
};
```

## QoS Level Mapping

MQTT QoS levels map to CLASP QoS as follows:

| MQTT QoS | CLASP QoS | Description |
|----------|-----------|-------------|
| 0 (At Most Once) | Q0 (Fire) | Best effort, no acknowledgment |
| 1 (At Least Once) | Q1 (Confirm) | Guaranteed delivery, may duplicate |
| 2 (Exactly Once) | Q2 (Commit) | Guaranteed exactly-once delivery |

Configure QoS in the bridge config:

```rust
let config = MqttBridgeConfig {
    qos: 1,  // Use QoS 1 for guaranteed delivery
    ..Default::default()
};
```

## Message Format Translation

### MQTT → CLASP

The bridge automatically parses MQTT payloads:

- **JSON**: `{"value": 25.5}` → `Value::Map` with parsed JSON
- **Number**: `"42"` → `Value::Float(42.0)`
- **Boolean**: `"true"` → `Value::Bool(true)`
- **String**: `"hello"` → `Value::String("hello")`
- **Binary**: Raw bytes → `Value::Bytes`

Example:

```rust
// MQTT publish: topic="sensors/temp", payload="25.5"
// → CLASP SET: address="/mqtt/sensors/temp", value=Value::Float(25.5)
```

### CLASP → MQTT

CLASP values are converted to MQTT payloads:

- **Numbers**: Serialized as string (e.g., `"42"`, `"3.14"`)
- **Booleans**: `"true"` or `"false"`
- **Strings**: UTF-8 bytes
- **Arrays/Maps**: JSON serialized
- **Bytes**: Raw binary payload

Example:

```rust
// CLASP SET: address="/mqtt/output/value", value=Value::Float(42.0)
// → MQTT publish: topic="output/value", payload="42"
```

## Subscription Patterns

The bridge subscribes to MQTT topics specified in `subscribe_topics`:

```rust
let config = MQTTBridgeConfig {
    subscribe_topics: vec![
        "sensors/#".to_string(),      // All sensor topics
        "actuators/+".to_string(),    // Single-level wildcard
        "home/+/temperature".to_string(), // Pattern matching
    ],
    ..Default::default()
};
```

**MQTT wildcards:**
- `+` = single-level wildcard (matches one segment)
- `#` = multi-level wildcard (matches any segments)

## Authentication

### Username/Password

```rust
let config = MqttBridgeConfig {
    broker_host: "broker.example.com".to_string(),
    broker_port: 1883,
    username: Some("myuser".to_string()),
    password: Some("mypass".to_string()),
    ..Default::default()
};
```

### TLS/SSL (Future)

TLS support is planned for secure MQTT connections (MQTT over TLS on port 8883).

## Retained Messages

MQTT retained messages are handled automatically:

- When the bridge subscribes to a topic with retained messages, it receives the last published value
- This value is translated to a CLASP SET message, providing state to late-joining CLASP clients

## Will Messages

MQTT Last Will and Testament (LWT) messages are supported:

```rust
// Configured via MqttOptions in the bridge implementation
// When bridge disconnects unexpectedly, broker publishes will message
```

## Example: IoT Sensor Integration

Complete example of MQTT sensors → CLASP → Web dashboard:

```rust
// 1. Start router
let router = Router::new(RouterConfig::default());
tokio::spawn(async move {
    router.serve_websocket("0.0.0.0:7330").await.unwrap();
});

// 2. Start MQTT bridge
let mqtt_config = MqttBridgeConfig {
    broker_host: "localhost".to_string(),
    broker_port: 1883,
    subscribe_topics: vec!["sensors/#".to_string()],
    namespace: "/mqtt".to_string(),
    ..Default::default()
};
let mut mqtt_bridge = MqttBridge::new(mqtt_config);
let mut mqtt_rx = mqtt_bridge.start().await?;

// 3. Forward MQTT messages to router
tokio::spawn(async move {
    while let Some(event) = mqtt_rx.recv().await {
        if let BridgeEvent::ToClasp(msg) = event {
            // Forward to router (implementation depends on router API)
        }
    }
});

// 4. CLASP clients can now subscribe to /mqtt/sensors/#
let client = ClaspBuilder::new("ws://localhost:7330")
    .name("Dashboard")
    .connect()
    .await?;

client.subscribe("/mqtt/sensors/#", |value, address| {
    println!("Sensor {} = {:?}", address, value);
}).await?;
```

## Testing

Run the MQTT integration tests:

```bash
# Start MQTT broker (mosquitto)
docker run -d -p 1883:1883 eclipse-mosquitto:latest

# Run tests
CLASP_TEST_BROKERS=1 cargo run --bin mqtt_integration_tests
```

Tests verify:
- MQTT → CLASP message translation
- CLASP → MQTT message translation
- Topic → address mapping
- QoS level configuration

## Troubleshooting

### "Connection failed"

- Verify MQTT broker is running: `telnet localhost 1883`
- Check broker host/port configuration
- Ensure network connectivity

### "Messages not received"

- Verify topic subscriptions match published topics
- Check namespace configuration
- Ensure bridge is connected (check logs)

### "Authentication failed"

- Verify username/password are correct
- Check broker authentication settings
- Ensure TLS is configured if required

## See Also

- [Bridge Setup Guide](../bridge-setup.md) - General bridge configuration
- [Protocol Mapping Guide](../protocol-mapping.md) - Address mapping examples
- [MQTT Bridge API](../../api/rust/bridge-api.md) - Full API reference
