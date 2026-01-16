# clasp-bridge

Protocol bridges for CLASP, enabling communication with external protocols like OSC, MIDI, MQTT, WebSocket, HTTP, Art-Net, and DMX.

## Supported Protocols

| Protocol | Feature Flag | Transport | Direction |
|----------|--------------|-----------|-----------|
| OSC | `osc` | UDP | Bidirectional |
| MIDI | `midi` | USB/Virtual | Bidirectional |
| MQTT | `mqtt` | TCP/TLS | Bidirectional |
| WebSocket | `websocket` | TCP | Bidirectional |
| HTTP | `http` | TCP | Bidirectional |
| Art-Net | `artnet` | UDP | Bidirectional |
| DMX | `dmx` | Serial | Output |
| Socket.IO | `socketio` | TCP | Bidirectional |

## Usage

```rust
use clasp_bridge::{OscBridge, OscBridgeConfig, Bridge, BridgeEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = OscBridgeConfig {
        bind_addr: "0.0.0.0:9000".to_string(),
        namespace: "/osc".to_string(),
        ..Default::default()
    };

    let mut bridge = OscBridge::new(config);
    let mut events = bridge.start().await?;

    while let Some(event) = events.recv().await {
        match event {
            BridgeEvent::ToClasp(msg) => {
                println!("Received: {:?}", msg);
            }
            BridgeEvent::Connected => println!("Bridge connected"),
            _ => {}
        }
    }

    Ok(())
}
```

## Bridge Trait

All bridges implement the `Bridge` trait:

```rust
#[async_trait]
pub trait Bridge: Send + Sync {
    fn config(&self) -> &BridgeConfig;
    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>>;
    async fn stop(&mut self) -> Result<()>;
    async fn send(&self, message: Message) -> Result<()>;
    fn is_running(&self) -> bool;
    fn namespace(&self) -> &str;
}
```

## Feature Flags

Enable only the protocols you need:

```toml
[dependencies]
clasp-bridge = { version = "0.1", default-features = false, features = ["osc", "mqtt"] }
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
