# CLASP

**Creative Low-Latency Application Streaming Protocol**

[![CI](https://github.com/lumencanvas/clasp/actions/workflows/ci.yml/badge.svg)](https://github.com/lumencanvas/clasp/actions/workflows/ci.yml)
[![Release](https://github.com/lumencanvas/clasp/actions/workflows/release.yml/badge.svg)](https://github.com/lumencanvas/clasp/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Website](https://img.shields.io/badge/website-clasp.to-teal)](https://clasp.to)

CLASP is a universal protocol bridge and signal router for creative applications. It unifies disparate protocols—OSC, MIDI, DMX, Art-Net, MQTT, WebSocket, HTTP—into a single, routable message system optimized for real-time performance.

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

## Features

- **Protocol Bridges**: OSC, MIDI, Art-Net, DMX, MQTT, WebSocket, Socket.IO, HTTP/REST
- **Signal Routing**: Wildcard patterns, transforms, aggregation
- **Low Latency**: QUIC-based transport with sub-millisecond overhead
- **Desktop App**: Visual bridge configuration and signal monitoring
- **CLI Tool**: Start servers and bridges from the command line
- **Embeddable**: Rust crates, WASM module, C FFI

## Quick Start

### Desktop App

Download the latest release for your platform:

- **macOS**: [CLASP Bridge.dmg](https://github.com/lumencanvas/clasp/releases/latest)
- **Windows**: [CLASP Bridge Setup.exe](https://github.com/lumencanvas/clasp/releases/latest)
- **Linux**: [clasp-bridge.AppImage](https://github.com/lumencanvas/clasp/releases/latest)

### CLI

```bash
# Install from source
cargo install --path crates/clasp-cli

# Start an OSC server on port 9000
clasp osc --port 9000

# Connect to an MQTT broker
clasp mqtt --host broker.local --port 1883

# Start HTTP REST API
clasp http --bind 0.0.0.0:3000

# Show all options
clasp --help
```

### As a Library

**Rust:**
```toml
# Cargo.toml
[dependencies]
clasp-core = "0.1"
clasp-bridge = { version = "0.1", features = ["osc", "mqtt"] }
```

**JavaScript/TypeScript:**
```bash
npm install @clasp-to/core
```

**Python:**
```bash
pip install clasp-to
```

### Rust Example

```rust
use clasp_bridge::{OscBridge, OscBridgeConfig, Bridge};

#[tokio::main]
async fn main() {
    let config = OscBridgeConfig {
        bind_addr: "0.0.0.0:9000".to_string(),
        ..Default::default()
    };

    let mut bridge = OscBridge::new(config);
    let mut events = bridge.start().await.unwrap();

    while let Some(event) = events.recv().await {
        println!("Received: {:?}", event);
    }
}
```

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

- [Getting Started](https://clasp.to/docs/getting-started)
- [Protocol Specification](https://clasp.to/docs/protocol)
- [API Reference](https://clasp.to/docs/api)
- [Examples](https://clasp.to/docs/examples)

## Project Structure

```
clasp/
├── crates/
│   ├── clasp-core/       # Core types, codec, state management
│   ├── clasp-transport/  # QUIC, TCP, WebSocket transports
│   ├── clasp-bridge/     # Protocol bridges (OSC, MIDI, MQTT, etc.)
│   ├── clasp-router/     # Message routing and pattern matching
│   ├── clasp-cli/        # Command-line interface
│   └── clasp-wasm/       # WebAssembly bindings
├── apps/
│   └── bridge/           # Electron desktop application
├── site/                 # Documentation website (Vue)
├── docs/                 # Markdown documentation
└── test-suite/           # Integration tests
```

## Supported Protocols

| Protocol | Direction | Features |
|----------|-----------|----------|
| **CLASP** | Bidirectional | Native protocol, QUIC transport, sub-ms latency |
| **OSC** | Bidirectional | UDP, bundles, all argument types |
| **MIDI** | Bidirectional | Notes, CC, program change, sysex |
| **Art-Net** | Bidirectional | DMX over Ethernet, multiple universes |
| **DMX** | Output | USB interfaces (FTDI, ENTTEC) |
| **MQTT** | Bidirectional | v3.1.1/v5, TLS, wildcards |
| **WebSocket** | Bidirectional | Client/server, JSON/binary |
| **Socket.IO** | Bidirectional | v4, rooms, namespaces |
| **HTTP** | Bidirectional | REST API, CORS, client/server |

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
