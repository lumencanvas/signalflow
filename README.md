<p align="center">
  <img src="assets/logo.svg" alt="CLASP Logo" width="120" />
</p>

<h1 align="center">CLASP</h1>

<p align="center">
  <strong>Creative Low-Latency Application Streaming Protocol</strong>
</p>

<p align="center">
  <a href="https://github.com/lumencanvas/clasp/actions/workflows/ci.yml"><img src="https://github.com/lumencanvas/clasp/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/clasp-cli"><img src="https://img.shields.io/crates/v/clasp-cli.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/@clasp-to/core"><img src="https://img.shields.io/npm/v/@clasp-to/core.svg" alt="npm"></a>
  <a href="https://pypi.org/project/clasp-to/"><img src="https://img.shields.io/pypi/v/clasp-to.svg" alt="PyPI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License"></a>
  <a href="https://clasp.to"><img src="https://img.shields.io/badge/website-clasp.to-teal" alt="Website"></a>
</p>

---

CLASP is a universal protocol bridge and signal router for creative applications. It unifies disparate protocols (OSC, MIDI, DMX, Art-Net, MQTT, WebSocket, HTTP) into a single, routable message system optimized for real-time performance.

## Why CLASP?

Creative projects often involve a chaotic mix of protocols:
- **Lighting** speaks DMX and Art-Net
- **Audio** software uses OSC and MIDI
- **IoT sensors** communicate via MQTT
- **Web interfaces** need WebSocket or HTTP
- **VJ software** has its own proprietary APIs

CLASP acts as the universal translator, letting everything talk to everything else through a unified address space.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  TouchOSC   │     │   Ableton   │     │  LED Strip  │
│  (OSC)      │     │   (MIDI)    │     │  (Art-Net)  │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                    ┌──────▼──────┐
                    │    CLASP    │
                    │   Router    │
                    └──────┬──────┘
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
│  Web UI     │     │  IoT Hub    │     │  Resolume   │
│ (WebSocket) │     │  (MQTT)     │     │  (OSC)      │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Install

### CLI

```bash
cargo install clasp-cli
```

### Libraries

| Platform | Package | Install |
|----------|---------|---------|
| **Rust** | [clasp-core](https://crates.io/crates/clasp-core) | `cargo add clasp-core` |
| **Rust** | [clasp-client](https://crates.io/crates/clasp-client) | `cargo add clasp-client` |
| **Rust** | [clasp-bridge](https://crates.io/crates/clasp-bridge) | `cargo add clasp-bridge` |
| **JavaScript** | [@clasp-to/core](https://www.npmjs.com/package/@clasp-to/core) | `npm install @clasp-to/core` |
| **Python** | [clasp-to](https://pypi.org/project/clasp-to/) | `pip install clasp-to` |

### Desktop App

Download the latest release for your platform:

- **macOS**: [CLASP Bridge.dmg](https://github.com/lumencanvas/clasp/releases/latest)
- **Windows**: [CLASP Bridge Setup.exe](https://github.com/lumencanvas/clasp/releases/latest)
- **Linux**: [clasp-bridge.AppImage](https://github.com/lumencanvas/clasp/releases/latest)

## Quick Start

### CLI Usage

```bash
# Start an OSC server on port 9000
clasp osc --port 9000

# Connect to an MQTT broker
clasp mqtt --host broker.local --port 1883

# Start HTTP REST API
clasp http --bind 0.0.0.0:3000

# Show all options
clasp --help
```

## CLASP-to-CLASP Examples

CLASP clients can communicate directly with each other through a CLASP router. Here are examples in each supported language:

### JavaScript/TypeScript

**Server (Node.js):**
```typescript
import { ClaspBuilder } from '@clasp-to/core';

// Connect to router
const server = await new ClaspBuilder('ws://localhost:7330')
  .withName('LED Controller')
  .connect();

// Listen for brightness changes
server.on('/lights/*/brightness', (value, address) => {
  console.log(`Setting ${address} to ${value}`);
  // Control actual LED hardware here
});

// Publish current state
await server.set('/lights/strip1/brightness', 0.8);
```

**Client (Browser or Node.js):**
```typescript
import { ClaspBuilder } from '@clasp-to/core';

const client = await new ClaspBuilder('ws://localhost:7330')
  .withName('Control Panel')
  .connect();

// Control the lights
await client.set('/lights/strip1/brightness', 0.5);

// Read current value
const brightness = await client.get('/lights/strip1/brightness');
console.log(`Current brightness: ${brightness}`);

// Subscribe to changes from other clients
client.on('/lights/**', (value, address) => {
  console.log(`${address} changed to ${value}`);
});
```

### Python

**Publisher:**
```python
import asyncio
from clasp import ClaspBuilder

async def main():
    client = await (
        ClaspBuilder('ws://localhost:7330')
        .with_name('Sensor Node')
        .connect()
    )

    # Publish sensor data
    while True:
        temperature = read_sensor()  # Your sensor code
        await client.set('/sensors/room1/temperature', temperature)
        await asyncio.sleep(1)

asyncio.run(main())
```

**Subscriber:**
```python
import asyncio
from clasp import ClaspBuilder

async def main():
    client = await (
        ClaspBuilder('ws://localhost:7330')
        .with_name('Dashboard')
        .connect()
    )

    # React to sensor updates
    @client.on('/sensors/*/temperature')
    def on_temperature(value, address):
        print(f'{address}: {value}°C')

    # Keep running
    await client.run()

asyncio.run(main())
```

### Rust

**Publisher:**
```rust
use clasp_client::{Clasp, ClaspBuilder};
use clasp_core::Value;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("Rust Publisher")
        .connect()
        .await?;

    // Set values that other clients can subscribe to
    client.set("/app/status", Value::String("running".into())).await?;
    client.set("/app/counter", Value::Int(42)).await?;

    // Stream high-frequency data
    for i in 0..100 {
        client.set("/app/position", Value::Float(i as f64 * 0.1)).await?;
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
    }

    client.close().await?;
    Ok(())
}
```

**Subscriber:**
```rust
use clasp_client::{Clasp, ClaspBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("Rust Subscriber")
        .connect()
        .await?;

    // Subscribe to all app signals
    let _unsub = client.subscribe("/app/**", |value, address| {
        println!("{} = {:?}", address, value);
    }).await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    client.close().await?;
    Ok(())
}
```

### Cross-Language Example

CLASP clients in different languages can seamlessly communicate:

```
┌────────────────────┐     ┌─────────────────┐     ┌────────────────────┐
│   Python Sensor    │     │  CLASP Router   │     │  JS Web Dashboard  │
│                    │────▶│  (port 7330)    │◀────│                    │
│ set('/temp', 23.5) │     │                 │     │ on('/temp', ...)   │
└────────────────────┘     └─────────────────┘     └────────────────────┘
                                   ▲
                                   │
                           ┌───────┴───────┐
                           │ Rust Actuator │
                           │               │
                           │ on('/temp',   │
                           │   adjust_hvac)│
                           └───────────────┘
```

## Features

- **Protocol Bridges**: OSC, MIDI, Art-Net, DMX, MQTT, WebSocket, Socket.IO, HTTP/REST
- **Signal Routing**: Wildcard patterns (`*`, `**`), transforms, aggregation
- **Low Latency**: WebSocket transport with sub-millisecond overhead
- **State Sync**: Automatic state synchronization between clients
- **Desktop App**: Visual bridge configuration and signal monitoring
- **CLI Tool**: Start servers and bridges from the command line
- **Embeddable**: Rust crates, WASM module, Python, JavaScript

## Performance

We believe in transparent benchmarking with honest methodology.

### Codec Benchmarks (In-Memory, Single Core)

These measure raw encode/decode speed—the **theoretical ceiling**, not system throughput:

| Protocol | Encode | Decode | Size | Notes |
|----------|--------|--------|------|-------|
| MQTT | 11.4M/s | 11.4M/s | 19 B | Minimal protocol |
| **CLASP** | **8M/s** | **11M/s** | **31 B** | Rich semantics |
| OSC | 4.5M/s | 5.7M/s | 24 B | UDP only |
| JSON-WS | ~2M/s | ~2M/s | ~80 B | Typical JSON overhead |

⚠️ **Important**: These are codec-only numbers (no network, no routing, no state). Real system throughput is 10-100x lower depending on features enabled.

### System Throughput (End-to-End)

Run `cargo run -p clasp-test-suite --bin real_benchmarks --release` for actual numbers including:
- End-to-end latency (pub → router → sub)
- Fanout to multiple subscribers
- Wildcard routing overhead
- State management costs

### Why Binary Encoding?

CLASP uses efficient binary encoding that is **55% smaller** than JSON:

```
JSON: {"type":"SET","address":"/test","value":0.5,...} → ~80 bytes
CLASP: [SET][flags][len][addr][value][rev]             → 31 bytes
```

### Feature Comparison

| Feature | CLASP | OSC | MQTT |
|---------|-------|-----|------|
| State synchronization | ✅ | ❌ | ❌ |
| Late-joiner support | ✅ | ❌ | ✅ |
| Typed signals (Param/Event/Stream) | ✅ | ❌ | ❌ |
| Wildcard subscriptions | ✅ | ❌ | ✅ |
| Clock sync | ✅ | ✅ | ❌ |
| Multi-protocol bridging | ✅ | ❌ | ❌ |

### Timing Guarantees

- **LAN (wired)**: Target ±1ms clock sync accuracy
- **WiFi**: Target ±5-10ms clock sync accuracy
- **Not suitable for**: Hard realtime, safety-critical, industrial control systems

CLASP is designed for **soft realtime** creative applications: VJ software, stage lighting, music production, interactive installations.

## Supported Protocols

| Protocol | Direction | Features |
|----------|-----------|----------|
| **CLASP** | Bidirectional | Native protocol, WebSocket transport, sub-ms latency |
| **OSC** | Bidirectional | UDP, bundles, all argument types |
| **MIDI** | Bidirectional | Notes, CC, program change, sysex |
| **Art-Net** | Bidirectional | DMX over Ethernet, multiple universes |
| **DMX** | Output | USB interfaces (FTDI, ENTTEC) |
| **MQTT** | Bidirectional | v3.1.1/v5, TLS, wildcards |
| **WebSocket** | Bidirectional | Client/server, JSON/binary |
| **Socket.IO** | Bidirectional | v4, rooms, namespaces |
| **HTTP** | Bidirectional | REST API, CORS, client/server |

## Transports

CLASP supports multiple network transports for different use cases:

| Transport | Use Case | Features |
|-----------|----------|----------|
| **WebSocket** | Web apps, cross-platform | Default transport, works everywhere, JSON or binary |
| **QUIC** | Native apps, mobile | TLS 1.3, 0-RTT, connection migration, multiplexed streams |
| **UDP** | Low-latency, local network | Minimal overhead, best for high-frequency data |
| **TCP** | Reliable delivery | For environments where UDP is blocked |
| **Serial** | Hardware integration | UART/RS-232 for embedded devices |
| **BLE** | Wireless sensors | Bluetooth Low Energy for IoT devices |
| **WebRTC** | P2P, browser-to-browser | NAT traversal, direct peer connections |

Enable transports with feature flags:
```bash
# Default (WebSocket + UDP + QUIC)
cargo add clasp-transport

# All transports
cargo add clasp-transport --features full

# Specific transports
cargo add clasp-transport --features "websocket,quic,serial"
```

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

- [Getting Started](https://clasp.to/docs/getting-started)
- [Protocol Specification](https://clasp.to/docs/protocol)
- [API Reference](https://clasp.to/docs/api)
- [Examples](https://clasp.to/docs/examples)

## Crates

| Crate | Description |
|-------|-------------|
| [clasp-core](https://crates.io/crates/clasp-core) | Core types, codec, state management |
| [clasp-transport](https://crates.io/crates/clasp-transport) | WebSocket, QUIC, TCP transports |
| [clasp-client](https://crates.io/crates/clasp-client) | High-level async client |
| [clasp-router](https://crates.io/crates/clasp-router) | Message routing and pattern matching |
| [clasp-bridge](https://crates.io/crates/clasp-bridge) | Protocol bridges (OSC, MIDI, MQTT, etc.) |
| [clasp-discovery](https://crates.io/crates/clasp-discovery) | mDNS/DNS-SD device discovery |
| [clasp-cli](https://crates.io/crates/clasp-cli) | Command-line interface |

## Building from Source

### Prerequisites

- Rust 1.75+
- Node.js 20+ (for desktop app)
- Platform-specific dependencies:
  - **Linux**: `libasound2-dev`, `libudev-dev`
  - **macOS**: Xcode Command Line Tools

### Build

```bash
# Clone the repository
git clone https://github.com/lumencanvas/clasp.git
cd clasp

# Build all Rust crates
cargo build --release

# Build desktop app
cd apps/bridge
npm install
npm run build
```

### Run Tests

```bash
cargo test --workspace
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

CLASP builds on the shoulders of giants:
- [Quinn](https://github.com/quinn-rs/quinn) - QUIC implementation
- [rosc](https://github.com/klingtnet/rosc) - OSC codec
- [midir](https://github.com/Boddlnagg/midir) - MIDI I/O
- [rumqttc](https://github.com/bytebeamio/rumqtt) - MQTT client

---

<p align="center">
  Maintained by <a href="https://lumencanvas.studio">LumenCanvas</a> | 2026
</p>
