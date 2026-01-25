# Feature Flags

Cargo feature flags for CLASP Rust crates.

## clasp-core

```toml
[dependencies]
clasp-core = { version = "3.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |
| `alloc` | Yes | Heap allocation |
| `serde` | No | Serde serialization |

### no_std Usage

```toml
clasp-core = { version = "3.1", default-features = false }
```

## clasp-client

```toml
[dependencies]
clasp-client = { version = "3.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `websocket` | Yes | WebSocket transport |
| `quic` | No | QUIC transport |
| `tls` | Yes | TLS support |
| `discovery` | Yes | Auto-discovery |
| `tokio` | Yes | Tokio runtime |

### Minimal Client

```toml
clasp-client = { version = "3.1", default-features = false, features = ["websocket"] }
```

### Full Client

```toml
clasp-client = { version = "3.1", features = ["quic", "discovery"] }
```

## clasp-router

```toml
[dependencies]
clasp-router = { version = "3.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `websocket` | Yes | WebSocket transport |
| `quic` | No | QUIC transport |
| `udp` | No | UDP transport |
| `tcp` | No | Raw TCP transport |
| `tls` | Yes | TLS support |
| `mdns` | Yes | mDNS discovery |
| `persistence` | No | State persistence |
| `metrics` | No | Prometheus metrics |
| `mqtt-server` | No | Accept MQTT clients directly |
| `osc-server` | No | Accept OSC clients via UDP |
| `full` | No | All features enabled |

### Production Router

```toml
clasp-router = { version = "3.1", features = [
    "quic",
    "persistence",
    "metrics"
] }
```

### Multi-Protocol Router

```toml
clasp-router = { version = "3.1", features = [
    "mqtt-server",
    "osc-server"
] }
```

## clasp-bridge

```toml
[dependencies]
clasp-bridge = { version = "3.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `osc` | No | OSC bridge |
| `midi` | No | MIDI bridge |
| `artnet` | No | Art-Net bridge |
| `dmx` | No | DMX bridge |
| `mqtt` | No | MQTT bridge |
| `sacn` | No | sACN bridge |
| `http` | No | HTTP bridge |
| `full` | No | All bridges |

### Select Bridges

```toml
clasp-bridge = { version = "3.1", features = ["osc", "midi"] }
```

### All Bridges

```toml
clasp-bridge = { version = "3.1", features = ["full"] }
```

## clasp-transport

```toml
[dependencies]
clasp-transport = { version = "3.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `websocket` | Yes | WebSocket |
| `quic` | No | QUIC |
| `udp` | No | UDP |
| `tcp` | No | Raw TCP |
| `webrtc` | No | WebRTC DataChannel |
| `serial` | No | Serial/UART |
| `ble` | No | Bluetooth LE |
| `full` | No | All transports |

## clasp-discovery

```toml
[dependencies]
clasp-discovery = { version = "3.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `mdns` | Yes | mDNS (Bonjour/Avahi) |
| `udp` | Yes | UDP broadcast |

## clasp-embedded

```toml
[dependencies]
clasp-embedded = { version = "3.1", default-features = false, features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `cortex-m` | No | Cortex-M support |
| `riscv` | No | RISC-V support |
| `esp32` | No | ESP32 support |

### ESP32 Project

```toml
clasp-embedded = { version = "3.1", default-features = false, features = ["esp32"] }
```

## Build Profiles

### Development

```toml
[profile.dev]
opt-level = 0
debug = true
```

### Release

```toml
[profile.release]
opt-level = 3
lto = true
```

### Embedded (Size Optimized)

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
```

## Conditional Compilation

Use feature flags in code:

```rust
#[cfg(feature = "quic")]
pub mod quic;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
pub struct Message { ... }
```

## See Also

- [Rust Installation](../../how-to/installation/rust-library.md)
- [Performance Tuning](../../how-to/advanced/performance-tuning.md)
- [Embedded Systems](../../use-cases/embedded-systems.md)
