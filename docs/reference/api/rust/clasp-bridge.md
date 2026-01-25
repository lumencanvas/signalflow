# clasp-bridge (Rust)

Protocol bridge implementations for CLASP.

## Overview

`clasp-bridge` provides bridges between CLASP and external protocols like OSC, MIDI, Art-Net, and MQTT.

```toml
[dependencies]
clasp-bridge = "3.1"

# Or select specific bridges
clasp-bridge = { version = "3.1", features = ["osc", "midi"] }
```

## Features

```toml
# All bridges
clasp-bridge = { version = "3.1", features = ["full"] }

# Individual bridges
clasp-bridge = { version = "3.1", features = [
    "osc",
    "midi",
    "artnet",
    "dmx",
    "mqtt",
    "sacn",
    "http"
] }
```

## OSC Bridge

### Basic Usage

```rust
use clasp_bridge::osc::{OscBridge, OscConfig};
use clasp_client::{Clasp, ClaspBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("OSC Bridge")
        .connect()
        .await?;

    let config = OscConfig {
        bind_addr: "0.0.0.0:8000".parse()?,
        target_addr: Some("192.168.1.100:9000".parse()?),
    };

    let bridge = OscBridge::new(client, config).await?;
    bridge.run().await
}
```

### Bidirectional

```rust
let config = OscConfig {
    bind_addr: "0.0.0.0:8000".parse()?,
    target_addr: Some("192.168.1.100:9000".parse()?),
    bidirectional: true,
};

let bridge = OscBridge::new(client, config).await?;

// OSC → CLASP: /1/fader1 → /osc/1/fader1
// CLASP → OSC: /osc/1/fader1 → /1/fader1
```

### Address Mapping

```rust
use clasp_bridge::osc::AddressMapper;

let mapper = AddressMapper::new()
    .map("/1/fader*", "/control/fader*")
    .map("/1/xy*", "/control/xy*")
    .prefix_unmapped("/osc");

let bridge = OscBridge::builder(client)
    .bind("0.0.0.0:8000")
    .target("192.168.1.100:9000")
    .mapper(mapper)
    .build()
    .await?;
```

## MIDI Bridge

### Basic Usage

```rust
use clasp_bridge::midi::{MidiBridge, MidiConfig};

let config = MidiConfig {
    device_name: "Launchpad X".into(),
    input_enabled: true,
    output_enabled: true,
};

let bridge = MidiBridge::new(client, config).await?;
bridge.run().await
```

### With Channel Filtering

```rust
let config = MidiConfig {
    device_name: "Controller".into(),
    channels: Some(vec![0, 1]),  // Only channels 1 and 2
    ..Default::default()
};
```

### Address Format

```
MIDI In → CLASP:
  Note:    /midi/{device}/note        → { note, velocity, channel }
  CC:      /midi/{device}/cc/{ch}/{cc} → value (0-127)
  PB:      /midi/{device}/pb/{ch}      → value (-8192 to 8191)

CLASP → MIDI Out:
  /midi/{device}/note        → Note message
  /midi/{device}/cc/{ch}/{cc} → CC message
```

## Art-Net Bridge

### Basic Usage

```rust
use clasp_bridge::artnet::{ArtNetBridge, ArtNetConfig};

let config = ArtNetConfig {
    bind_addr: "0.0.0.0:6454".parse()?,
    broadcast_addr: "255.255.255.255:6454".parse()?,
};

let bridge = ArtNetBridge::new(client, config).await?;
bridge.run().await
```

### Multiple Universes

```rust
let config = ArtNetConfig {
    bind_addr: "0.0.0.0:6454".parse()?,
    universes: vec![0, 1, 2, 3],  // Universes 0-3
    ..Default::default()
};

// Addresses: /artnet/{net}/{subnet}/{universe}/{channel}
// Example: /artnet/0/0/0/1 = Universe 0, Channel 1
```

### Output Only

```rust
let config = ArtNetConfig {
    bind_addr: "0.0.0.0:6454".parse()?,
    input_enabled: false,
    output_enabled: true,
    output_targets: vec!["192.168.1.50:6454".parse()?],
    ..Default::default()
};
```

## DMX Bridge

### Serial DMX

```rust
use clasp_bridge::dmx::{DmxBridge, DmxConfig, DmxInterface};

let config = DmxConfig {
    interface: DmxInterface::Serial {
        port: "/dev/ttyUSB0".into(),
        baud: 250000,
    },
    universe: 0,
};

let bridge = DmxBridge::new(client, config).await?;
```

### USB DMX (FTDI)

```rust
let config = DmxConfig {
    interface: DmxInterface::Ftdi {
        device_index: 0,
    },
    universe: 0,
};
```

## MQTT Bridge

### Basic Usage

```rust
use clasp_bridge::mqtt::{MqttBridge, MqttConfig};

let config = MqttConfig {
    host: "localhost".into(),
    port: 1883,
    client_id: "clasp-bridge".into(),
    topics: vec!["sensors/#".into(), "control/#".into()],
};

let bridge = MqttBridge::new(client, config).await?;
bridge.run().await
```

### With Authentication

```rust
let config = MqttConfig {
    host: "mqtt.example.com".into(),
    port: 8883,
    username: Some("user".into()),
    password: Some("pass".into()),
    tls_enabled: true,
    ..Default::default()
};
```

### Topic Mapping

```rust
// MQTT → CLASP: sensors/temp → /mqtt/sensors/temp
// CLASP → MQTT: /mqtt/control/led → control/led

let mapper = MqttMapper::new()
    .incoming_prefix("/mqtt")
    .outgoing_strip_prefix("/mqtt");
```

## sACN Bridge

### Basic Usage

```rust
use clasp_bridge::sacn::{SacnBridge, SacnConfig};

let config = SacnConfig {
    universes: vec![1, 2, 3],
    source_name: "CLASP Bridge".into(),
    priority: 100,
};

let bridge = SacnBridge::new(client, config).await?;
```

## HTTP Bridge

### REST API Bridge

```rust
use clasp_bridge::http::{HttpBridge, HttpConfig};

let config = HttpConfig {
    bind_addr: "0.0.0.0:3000".parse()?,
    cors_enabled: true,
};

let bridge = HttpBridge::new(client, config).await?;

// GET  /api/state/{address} - Get value
// PUT  /api/state/{address} - Set value
// POST /api/event/{address} - Emit event
// GET  /api/subscribe/{address} - SSE subscription
```

## Bridge Trait

Create custom bridges:

```rust
use clasp_bridge::{Bridge, BridgeConfig};
use async_trait::async_trait;

pub struct MyBridge {
    client: Client,
    config: MyConfig,
}

#[async_trait]
impl Bridge for MyBridge {
    type Config = MyConfig;

    async fn new(client: Client, config: Self::Config) -> Result<Self> {
        Ok(Self { client, config })
    }

    async fn run(&self) -> Result<()> {
        // Main bridge loop
        loop {
            // Handle external messages
            // Forward to CLASP client
        }
    }

    async fn shutdown(&self) -> Result<()> {
        // Cleanup
        Ok(())
    }
}
```

## Error Handling

```rust
use clasp_bridge::Error;

match bridge.run().await {
    Ok(()) => println!("Bridge stopped"),
    Err(Error::ConnectionError(e)) => eprintln!("Connection failed: {}", e),
    Err(Error::DeviceNotFound(name)) => eprintln!("Device not found: {}", name),
    Err(Error::ProtocolError(e)) => eprintln!("Protocol error: {}", e),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## See Also

- [clasp-client](clasp-client.md) - Client library
- [Bridge Reference](../../bridges/) - Protocol-specific details
- [Custom Bridge](../../../how-to/advanced/custom-bridge.md)
