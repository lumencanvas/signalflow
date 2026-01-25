# Getting Started with CLASP

CLASP (Creative Low-Latency Application Streaming Protocol) is a universal protocol bridge for creative applications. This guide will help you get up and running.

## Installation

### Desktop App

The easiest way to get started is with the CLASP Bridge desktop app:

- **macOS**: Download [CLASP Bridge.dmg](https://github.com/lumencanvas/clasp/releases/latest)
- **Windows**: Download [CLASP Bridge Setup.exe](https://github.com/lumencanvas/clasp/releases/latest)
- **Linux**: Download [clasp-bridge.AppImage](https://github.com/lumencanvas/clasp/releases/latest)

### CLI Tool

Install the command-line tool via Cargo:

```bash
cargo install clasp-cli
```

Or build from source:

```bash
git clone https://github.com/lumencanvas/clasp.git
cd clasp
cargo install --path crates/clasp-cli
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
clasp-core = "3.1"
clasp-bridge = { version = "3.1", features = ["osc", "mqtt"] }
```

## Quick Start

### Using the Desktop App

1. Launch CLASP Bridge
2. Click **ADD** in the sidebar to start a server
3. Select your protocol (OSC, MQTT, WebSocket, etc.)
4. Configure the settings and click **START SERVER**
5. Use the **Monitor** tab to see incoming signals

### Using the CLI

```bash
# Start an OSC server
clasp osc --port 9000

# Start an MQTT connection
clasp mqtt --host localhost --port 1883

# Start a WebSocket server
clasp websocket --mode server --url 0.0.0.0:8080

# Start an HTTP REST API
clasp http --bind 0.0.0.0:3000
```

### Using the Library

```rust
use clasp_bridge::{OscBridge, OscBridgeConfig, Bridge, BridgeEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create an OSC bridge
    let config = OscBridgeConfig {
        bind_addr: "0.0.0.0:9000".to_string(),
        namespace: "/osc".to_string(),
        ..Default::default()
    };

    let mut bridge = OscBridge::new(config);
    let mut events = bridge.start().await?;

    // Handle incoming messages
    while let Some(event) = events.recv().await {
        match event {
            BridgeEvent::ToClasp(msg) => {
                println!("Received: {:?}", msg);
            }
            BridgeEvent::Connected => {
                println!("Bridge connected");
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Next Steps

- [Concepts](../concepts/) - Understand addresses, signals, and routing
- [Protocols](../protocols/) - Protocol-specific documentation
- [Examples](../examples/) - Real-world usage examples
- [API Reference](../api/) - Complete API documentation
