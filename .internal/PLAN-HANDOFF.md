# CLASP Project Master Plan

**Last Updated:** 2026-01-22
**Version:** 0.1.8
**Status:** Active Development

---

## Quick Context

CLASP (Creative Low-Latency Application Streaming Protocol) is a universal protocol bridge for creative applications. This document consolidates all internal planning.

---

## Repository Structure

```
clasp/
├── crates/                    # Core Rust libraries
│   ├── clasp-core/           # Types, codec, addresses (foundational)
│   ├── clasp-transport/      # WebSocket, QUIC, UDP, BLE, Serial
│   ├── clasp-router/         # Message routing, sessions, state (SERVER)
│   ├── clasp-client/         # Async client library
│   ├── clasp-bridge/         # Protocol bridges (OSC, MIDI, DMX, MQTT)
│   ├── clasp-discovery/      # mDNS + UDP broadcast
│   ├── clasp-embedded/       # no_std client + server (~3.6KB)
│   ├── clasp-wasm/           # Browser WebAssembly bindings
│   └── clasp-cli/            # User CLI tool
├── tools/                     # Binary applications
│   ├── clasp-router-server/  # Standalone router binary
│   └── clasp-service/        # Background service
├── bindings/                  # Language bindings
│   ├── js/                   # @clasp-to/core (npm)
│   └── python/               # clasp-to (PyPI)
├── apps/                      # Desktop applications
│   └── bridge/               # Electron desktop app
├── site/                      # clasp.to website
├── deploy/                    # Deployment configs
│   └── relay/                # Standalone relay (uses published crates)
├── test-suite/               # Integration tests
├── docs/                     # Documentation
└── examples/                 # Usage examples
```

---

## Published Packages

| Platform | Package | Registry | Version |
|----------|---------|----------|---------|
| Rust | clasp-core | crates.io | 0.1.8 |
| Rust | clasp-transport | crates.io | 0.1.8 |
| Rust | clasp-router | crates.io | 0.1.8 |
| Rust | clasp-client | crates.io | 0.1.8 |
| Rust | clasp-bridge | crates.io | 0.1.8 |
| Rust | clasp-discovery | crates.io | 0.1.8 |
| Rust | clasp-embedded | crates.io | 0.1.8 |
| Rust | clasp-cli | crates.io | 0.1.8 |
| JavaScript | @clasp-to/core | npm | 0.1.8 |
| Python | clasp-to | PyPI | 0.1.0 |

---

## Workstream Tracking

### ✅ Completed

- [x] Binary encoding (55% smaller, 4-7x faster)
- [x] Embedded client + server (3.6KB, no_std)
- [x] v2 → v3 migration (all references removed)
- [x] Standalone relay deployment (uses published crates)
- [x] WebSocket subprotocol updated to `clasp`
- [x] Protocol spec cleaned up (single version)
- [x] Architecture documentation

### 🔄 In Progress

- [ ] Server-in-application examples (Rust, Node.js, Python)
- [ ] Version coordination across packages
- [ ] Desktop app protocol updates

### 📋 Planned

- [ ] Pluggable state storage (Redis, MongoDB, SQLite)
- [ ] Memory configuration (limits for constrained environments)
- [ ] Native observability (OpenTelemetry, Prometheus)
- [ ] Node.js server package (@clasp-to/server)
- [ ] Zephyr RTOS compatibility research

---

## Critical Tasks

### 1. Version Coordination (HIGH PRIORITY)

All packages must stay in sync:

```bash
# Check versions match
grep 'version = "0.1' crates/*/Cargo.toml
grep '"version"' bindings/*/package.json

# Update all versions
cargo set-version 0.1.9 --workspace
# Then update JS/Python manually
```

**Version bump checklist:**
- [ ] Update Cargo.toml version (workspace)
- [ ] Update package.json (@clasp-to/core)
- [ ] Update pyproject.toml (clasp-to)
- [ ] Update apps/bridge/package.json
- [ ] Run all tests
- [ ] Publish to registries

### 2. Desktop App (apps/bridge)

**Status:** Needs protocol update verification

**Tasks:**
- [ ] Verify WebSocket connects with `clasp` subprotocol
- [ ] Test message encoding/decoding with binary format
- [ ] Update any UI that references v2/v3
- [ ] Test with latest clasp-router

**Files to check:**
- `apps/bridge/electron/main.js` ✅ (updated to `clasp`)
- `apps/bridge/src/app.js`
- `apps/bridge/package.json` (dependencies)

### 3. Server-in-Application Examples

Create examples for embedding CLASP server in your own application:

**Rust:** `examples/rust/embedded-server.rs` ✅ (created)

**Node.js:** `examples/js/embedded-server.js` (TODO)
```javascript
const { createServer } = require('@clasp-to/server');

const server = createServer({ port: 7330 });

// Publish from your app
setInterval(() => {
  server.set('/sensors/cpu', getCpuUsage());
}, 1000);

server.start();
```

**Python:** `examples/python/embedded_server.py` (TODO)
```python
from clasp import Server

server = Server(port=7330)

# Your app logic
@server.on_connect
def on_connect(session):
    print(f"Client connected: {session.name}")

server.run()
```

### 4. Node.js Server Package

**Goal:** Allow running CLASP server from Node.js

**Options:**
1. **NAPI-RS** - Native Rust addon (best performance)
2. **WASM** - Cross-platform (slightly slower)

**Package:** `@clasp-to/server`

**API Design:**
```javascript
const { ClaspServer, MemoryStorage, RedisStorage } = require('@clasp-to/server');

const server = new ClaspServer({
  port: 7330,
  name: 'My Server',
  storage: new RedisStorage('redis://localhost'),
});

server.on('connect', (session) => { ... });
server.on('set', (address, value, session) => { ... });

await server.start();
```

### 5. Pluggable State Storage

**Current:** In-memory HashMap

**Proposed trait:**
```rust
#[async_trait]
pub trait StateStorage: Send + Sync {
    async fn get(&self, address: &str) -> Option<ParamState>;
    async fn set(&self, address: &str, state: ParamState) -> Result<()>;
    async fn get_matching(&self, pattern: &str) -> Vec<(String, ParamState)>;
    async fn snapshot(&self) -> Vec<ParamValue>;
    async fn clear(&self) -> Result<()>;
}
```

**Implementations:**
- `MemoryStorage` (default)
- `RedisStorage` (feature-gated)
- `MongoStorage` (feature-gated)
- `SqliteStorage` (feature-gated)

### 6. Memory Configuration

```rust
pub struct RouterConfig {
    // Existing
    pub name: String,
    pub security_mode: SecurityMode,
    pub max_sessions: usize,
    pub session_timeout: u64,
    pub features: Vec<String>,
    pub max_subscriptions_per_session: usize,
    
    // New limits
    pub max_state_entries: Option<usize>,    // None = unlimited
    pub max_message_size: usize,              // Default 64KB
    pub max_queue_per_session: usize,         // Default 1000
    pub state_ttl: Option<Duration>,          // State expiration
}
```

### 7. Observability

**Current:** Basic tracing logs

**Proposed:**
```rust
pub trait RouterMetrics: Send + Sync {
    fn on_connect(&self, session_id: &str);
    fn on_disconnect(&self, session_id: &str, duration: Duration);
    fn on_message(&self, msg_type: &str, size: usize, latency_us: u64);
    fn on_fanout(&self, address: &str, subscriber_count: usize);
    fn on_state_update(&self, address: &str);
    fn on_error(&self, category: &str, message: &str);
}
```

**Built-in implementations:**
- `TracingMetrics` (default - uses tracing crate)
- `PrometheusMetrics` (feature: metrics-prometheus)
- `OpenTelemetryMetrics` (feature: metrics-otel)

---

## Embedded Architecture

### Transport-Agnostic Design

`clasp-embedded` provides encoding/decoding only. YOU provide transport:

```rust
// ESP32 with WebSocket
let frame = client.prepare_set("/sensor/temp", Value::Float(25.5));
websocket.send(frame);  // Your WebSocket implementation

// ESP32 with HTTP POST
http.post("https://relay.clasp.to/frames", frame);

// ESP32 with MQTT
mqtt.publish("clasp/frames", frame);
```

### Memory Budget

| Component | Size |
|-----------|------|
| Client | 3,600 bytes |
| StateCache (32 entries) | 2,824 bytes |
| MiniRouter | ~4,000 bytes |
| **Total** | **~10KB max** |

### Zephyr Compatibility (Research)

Requirements:
- `no_std` async runtime (embassy or smol)
- Zephyr network stack bindings
- Possibly cargo-zephyr

---

## Protocol Notes

### WebSocket Subprotocol

**Value:** `clasp` (NOT `clasp.v2` or `clasp.v3`)

All implementations must use this:
```javascript
new WebSocket(url, 'clasp');
```

### Binary Encoding

All messages use positional binary encoding:
- Magic byte: 0x53 ('S')
- Flags: QoS, timestamp, encryption, compression
- Payload: Type byte + positional fields

No MessagePack, no JSON over WebSocket.

### Message Types

| Code | Type | Description |
|------|------|-------------|
| 0x01 | HELLO | Client → Server handshake |
| 0x02 | WELCOME | Server → Client handshake response |
| 0x10 | SUBSCRIBE | Pattern subscription |
| 0x11 | UNSUBSCRIBE | Cancel subscription |
| 0x20 | PUBLISH | Fire-and-forget event |
| 0x21 | SET | State update with revision |
| 0x23 | SNAPSHOT | State snapshot (chunked) |
| 0x30 | BUNDLE | Atomic message group |
| 0x41 | PING | Keepalive |
| 0x42 | PONG | Keepalive response |

---

## Deployment

### Standalone Relay

The `deploy/relay/` directory contains a standalone project that uses **published crates** from crates.io:

```bash
cd deploy/relay
docker build -t clasp-relay .
docker run -p 7330:7330 clasp-relay
```

This does NOT require the monorepo - it pulls from crates.io.

### Development Relay

To build with local crates (from repo root):

```bash
docker build -f deploy/relay/Dockerfile.dev -t clasp-relay-dev .
```

---

## Testing Checklist

Before any release:

```bash
# Core tests
cargo test --workspace

# Integration tests
cargo run -p clasp-test-suite --bin run-all-tests

# Embedded tests
cargo run -p clasp-test-suite --bin embedded-tests --release

# Protocol tests
cargo run -p clasp-test-suite --bin protocol-tests

# Benchmarks
cargo run -p clasp-test-suite --bin real_benchmarks --release
```

---

## Documentation Updates Needed

- [ ] Update clasp.to website for new protocol
- [ ] Update playground for `clasp` subprotocol
- [ ] Remove any v2/v3 comparison language
- [ ] Add embedded transport examples
- [ ] Add server embedding examples
- [ ] Architecture diagram

---

## Research Topics

### Zephyr RTOS
- Embassy async runtime
- Zephyr network stack
- cargo-zephyr tooling

### Edge Computing
- WASM on microcontrollers (Wasm3, WAMR)
- Edge inference integration

### Distributed State
- CRDTs for conflict-free replication
- Raft consensus for critical state
- Event sourcing patterns

---

## Contacts & Resources

- **Repository:** github.com/lumencanvas/clasp
- **Website:** clasp.to
- **npm:** @clasp-to/core
- **PyPI:** clasp-to
- **crates.io:** clasp-*

---

*This document should be updated with each significant change.*
