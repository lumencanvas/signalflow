# MQTT Bridge

The MQTT bridge connects to MQTT brokers for IoT messaging, home automation, and distributed sensor networks.

## Configuration

```rust
use clasp_bridge::{MqttBridge, MqttBridgeConfig};

let config = MqttBridgeConfig {
    broker_host: "localhost".to_string(),
    broker_port: 1883,
    client_id: "clasp-bridge".to_string(),
    username: Some("user".to_string()),
    password: Some("pass".to_string()),
    subscribe_topics: vec!["sensors/#".to_string(), "controls/#".to_string()],
    qos: 1,
    namespace: "/mqtt".to_string(),
};

let bridge = MqttBridge::new(config);
```

### CLI Usage

```bash
# Connect to local broker
clasp mqtt --host localhost --port 1883

# Connect with authentication
clasp mqtt --host broker.example.com --user myuser --pass mypass

# Subscribe to specific topics
clasp mqtt --host localhost --topic "sensors/#" --topic "home/+"
```

### Desktop App

1. Click **ADD** in the sidebar
2. Select **MQTT Broker**
3. Enter broker host and port
4. Configure topics (comma-separated)
5. Click **START SERVER**

## Topic Mapping

MQTT topics are mapped to CLASP addresses:

| MQTT Topic | CLASP Address |
|------------|---------------|
| `sensors/temp` | `/mqtt/sensors/temp` |
| `home/living/light` | `/mqtt/home/living/light` |
| `device/123/status` | `/mqtt/device/123/status` |

## Payload Parsing

The bridge automatically detects and converts payloads:

| Payload | Detected Type | CLASP Value |
|---------|---------------|-------------|
| `42` | Number | `Int(42)` |
| `3.14` | Number | `Float(3.14)` |
| `true` | Boolean | `Bool(true)` |
| `"hello"` | String | `String("hello")` |
| `{"x": 1}` | JSON | `Map` |
| Binary data | Bytes | `Bytes` |

## QoS Levels

| Level | Meaning | Use Case |
|-------|---------|----------|
| 0 | At most once | Fire-and-forget, sensor data |
| 1 | At least once | Important messages |
| 2 | Exactly once | Critical commands |

## Examples

### Receiving MQTT Messages

```rust
use clasp_bridge::{MqttBridge, MqttBridgeConfig, Bridge, BridgeEvent};

#[tokio::main]
async fn main() {
    let config = MqttBridgeConfig {
        broker_host: "localhost".to_string(),
        broker_port: 1883,
        subscribe_topics: vec!["#".to_string()], // Subscribe to all
        ..Default::default()
    };

    let mut bridge = MqttBridge::new(config);
    let mut events = bridge.start().await.unwrap();

    while let Some(event) = events.recv().await {
        if let BridgeEvent::ToClasp(msg) = event {
            println!("Received: {:?}", msg);
        }
    }
}
```

### Publishing to MQTT

```rust
use clasp_bridge::{MqttBridge, Bridge};
use clasp_core::{Message, SetMessage, Value};

async fn publish(bridge: &MqttBridge) {
    let msg = Message::Set(SetMessage {
        address: "/mqtt/home/light/brightness".to_string(),
        value: Value::Int(75),
        revision: None,
        lock: false,
        unlock: false,
    });

    bridge.send(msg).await.unwrap();
}
```

## Common Brokers

| Broker | Default Port | Notes |
|--------|--------------|-------|
| Mosquitto | 1883 | Open source, lightweight |
| HiveMQ | 1883 | Enterprise features |
| EMQX | 1883 | Scalable, clustering |
| Home Assistant | 1883 | Smart home integration |
| AWS IoT | 8883 (TLS) | Cloud IoT platform |
