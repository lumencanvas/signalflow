# Rust Library

Add CLASP to Rust projects.

## Add Dependencies

For a client application:

```toml
[dependencies]
clasp-client = "3.1"
clasp-core = "3.1"
tokio = { version = "1", features = ["full"] }
```

For building a router:

```toml
[dependencies]
clasp-router = "3.1"
clasp-core = "3.1"
tokio = { version = "1", features = ["full"] }
```

## Basic Client

```rust
use clasp_client::{ClaspBuilder, Clasp};
use clasp_core::Value;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("My App")
        .connect()
        .await?;

    // Set a value
    client.set("/path", Value::Float(42.0)).await?;

    // Subscribe
    client.subscribe("/sensors/**", |value, addr| {
        println!("{} = {:?}", addr, value);
    }).await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    client.close().await?;
    Ok(())
}
```

## Feature Flags

### clasp-transport

```toml
# Default (WebSocket + UDP + QUIC)
clasp-transport = "3.1"

# Specific transports
clasp-transport = { version = "3.1", features = ["websocket", "quic"] }

# All transports
clasp-transport = { version = "3.1", features = ["full"] }
```

Available features:
- `websocket` — WebSocket transport
- `quic` — QUIC transport
- `udp` — UDP transport
- `serial` — Serial/UART transport
- `ble` — Bluetooth Low Energy
- `webrtc` — WebRTC DataChannel

### clasp-bridge

```toml
# Specific bridges
clasp-bridge = { version = "3.1", features = ["osc", "midi"] }

# All bridges
clasp-bridge = { version = "3.1", features = ["full"] }
```

Available features:
- `osc` — OSC bridge
- `midi` — MIDI bridge
- `artnet` — Art-Net bridge
- `dmx` — DMX bridge
- `mqtt` — MQTT bridge

## Embedded (no_std)

For microcontrollers:

```toml
[dependencies]
clasp-embedded = { version = "3.1", features = ["client"] }
```

```rust
#![no_std]

use clasp_embedded::{Client, Value};

let mut client = Client::new();
let frame = client.prepare_set("/sensor/temp", Value::Float(25.5));
// Send frame via your transport
```

## Next Steps

- [Connect a Client](../connections/connect-client.md)
- [Embed a Router](../advanced/embed-router.md)
- [Rust API Reference](../../reference/api/rust/clasp-client.md)
