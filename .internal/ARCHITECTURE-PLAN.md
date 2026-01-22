# CLASP Architecture Analysis & Improvement Plan

**Created:** 2026-01-22
**Status:** PLANNING

---

## Current State Analysis

### Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                         │
├─────────────────────────────────────────────────────────────────┤
│  tools/clasp-router-server    CLI server binary                 │
│  tools/clasp-service          Background service                │
│  crates/clasp-cli             User CLI tool                     │
│  apps/bridge                  Desktop app (Electron)            │
├─────────────────────────────────────────────────────────────────┤
│                         LIBRARY LAYER                            │
├─────────────────────────────────────────────────────────────────┤
│  clasp-router   Message routing, sessions, subscriptions        │
│  clasp-client   Async client (connect, subscribe, publish)      │
│  clasp-bridge   Protocol bridges (OSC, MIDI, DMX, MQTT...)      │
│  clasp-discovery mDNS + UDP broadcast discovery                 │
├─────────────────────────────────────────────────────────────────┤
│                        TRANSPORT LAYER                           │
├─────────────────────────────────────────────────────────────────┤
│  clasp-transport  WebSocket, QUIC, UDP, BLE, Serial, WebRTC     │
├─────────────────────────────────────────────────────────────────┤
│                          CORE LAYER                              │
├─────────────────────────────────────────────────────────────────┤
│  clasp-core       Types, codec (v3 binary), addresses, security │
├─────────────────────────────────────────────────────────────────┤
│                       EMBEDDED LAYER                             │
├─────────────────────────────────────────────────────────────────┤
│  clasp-embedded   no_std client + server (brings own transport) │
└─────────────────────────────────────────────────────────────────┘
```

### What Each Crate Does

| Crate | Role | Depends On | Notes |
|-------|------|------------|-------|
| **clasp-core** | Protocol foundation | (none) | Types, codec, addresses, frame format. ~90KB compiled. |
| **clasp-transport** | Network I/O | clasp-core | WebSocket, QUIC, UDP, BLE, Serial. Feature-gated. |
| **clasp-router** | Message routing | clasp-core, clasp-transport | Sessions, subscriptions, state management. IS a server library. |
| **clasp-client** | Client API | clasp-core, clasp-transport | Builder pattern, async subscriptions, reconnect logic. |
| **clasp-bridge** | Protocol translation | clasp-core | OSC↔CLASP, MIDI↔CLASP, DMX↔CLASP, MQTT↔CLASP. |
| **clasp-discovery** | Service discovery | clasp-core | mDNS (_clasp._tcp), UDP broadcast. |
| **clasp-embedded** | Constrained devices | (none) | no_std, 3.6KB RAM. YOU provide transport. |
| **clasp-wasm** | Browser bindings | clasp-core, clasp-client | WebSocket-only in browser. |
| **clasp-cli** | User CLI | All above | `clasp publish`, `clasp subscribe`, etc. |

---

## User Questions Answered

### Q1: How does embedded connect to cloud?

**Current state:** `clasp-embedded` is transport-agnostic. It provides:
- Message encoding/decoding (v3 binary format)
- State cache
- Client/Server state machines

**YOU provide the transport bytes.** This is intentional for maximum flexibility.

```rust
// Example: ESP32 with WiFi HTTP
use clasp_embedded::{Client, encode_set_frame, Value};
use esp_wifi::http::HttpClient;

let mut client = Client::new();
let frame = client.prepare_set("/sensor/temp", Value::Float(25.5));

// Send via HTTP POST (you implement this)
http_client.post("https://relay.clasp.to/frames", frame);

// Or via WebSocket (you implement the WS connection)
ws_connection.send(frame);
```

**Supported transports for embedded:**
- **HTTP POST** - Works anywhere, stateless
- **WebSocket** - Works anywhere, bidirectional
- **UDP** - Low latency, local network only
- **MQTT** - Through MQTT broker as transport
- **BLE** - Short range, peer-to-peer
- **Raw TCP** - Simple, works anywhere

### Q2: Should there be separate client/server libraries?

**Current state:**
- `clasp-client` = Client-only library ✓
- `clasp-router` = Server library (routes messages, manages state)

**Confusion point:** `clasp-router` is both a library AND has a binary in `tools/clasp-router-server`.

**Recommendation:** Rename for clarity:
- `clasp-router` → `clasp-server` (the library)
- `clasp-router-server` → `clasp-router` (the binary)

### Q3: Should embedded support bridges?

**No.** Bridges are heavyweight (depend on protocol libraries like `rosc`, `midir`).

Embedded should stay minimal. If you need bridging on ESP32:
1. Connect to a CLASP router
2. Run bridges on the router (cloud/desktop)

### Q4: What's the difference between running a bridge vs a server?

```
                    ┌──────────────────┐
                    │  CLASP Router    │
                    │  (clasp-server)  │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
   ┌────▼────┐          ┌────▼────┐         ┌────▼────┐
   │ OSC     │          │ MIDI    │         │ MQTT    │
   │ Bridge  │          │ Bridge  │         │ Bridge  │
   └────┬────┘          └────┬────┘         └────┬────┘
        │                    │                    │
   ┌────▼────┐          ┌────▼────┐         ┌────▼────┐
   │TouchOSC │          │Ableton  │         │Sensors  │
   └─────────┘          └─────────┘         └─────────┘
```

**A bridge:**
1. Connects to a CLASP router as a client
2. Translates between external protocol ↔ CLASP
3. Does NOT route messages itself

**A router (server):**
1. Accepts connections from clients
2. Routes messages between clients based on subscriptions
3. Manages state (parameters, revisions)

### Q5: Where does clasp-transport fit in?

`clasp-transport` provides transport implementations that both client and server use:

```rust
// In clasp-client:
use clasp_transport::websocket::connect;
let conn = connect("wss://relay.clasp.to").await?;

// In clasp-router:
use clasp_transport::websocket::serve;
serve("0.0.0.0:7330", handler).await?;
```

### Q6: WASM on microcontrollers?

**Yes, technically possible!** Runtimes:
- **Wasm3** - Interprets WASM, 64KB footprint
- **WAMR** - WebAssembly Micro Runtime
- **Wasmi** - Pure Rust interpreter

**But not recommended for CLASP:**
- WASM adds overhead
- `clasp-embedded` is already Rust → native
- Use `clasp-embedded` directly on microcontrollers

### Q7: Zephyr compatibility?

**Possible with work.** Requirements:
1. `clasp-embedded` already `no_std`
2. Need Zephyr-compatible async runtime (smol/embassy)
3. Need Zephyr network stack bindings

**Current blockers:**
- No Zephyr transport implementations
- Would need `cargo-zephyr` or similar

---

## Issues & Fixes

### Issue 1: Deploy Failed

**Root cause:** Dockerfile uses `COPY Cargo.toml` but DigitalOcean builds from `deploy/relay/` context, not repo root.

**Fix:** The app.yaml has `source_dir: /` which should work. The actual error:
```
failed to get files: lstat /.app_platform_workspace/deploy/relay/Cargo.toml: no such file
```

This happens because DO copies files to a different location. Need to update Dockerfile path or use a standalone deployment.

### Issue 2: Dockerfile references wrong package

```dockerfile
RUN cargo build --release -p clasp-router-server
```

The package name is `clasp-router-server` (correct), binary name is `clasp-router` (correct).

### Fix for Deploy

Create standalone deployment that uses published crates:

```toml
# deploy/relay/Cargo.toml (NEW FILE)
[package]
name = "clasp-relay"
version = "0.1.0"
edition = "2021"

[dependencies]
# Use published crates
clasp-core = "0.1"
clasp-router = "0.1"
clasp-transport = { version = "0.1", features = ["websocket"] }

tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[[bin]]
name = "clasp-relay"
path = "src/main.rs"
```

---

## Architecture Improvements

### 1. Pluggable State Storage

**Current:** In-memory HashMap.

**Proposed:** Trait-based storage backend.

```rust
// In clasp-router (or new clasp-storage crate)
#[async_trait]
pub trait StateStorage: Send + Sync {
    async fn get(&self, address: &str) -> Option<ParamState>;
    async fn set(&self, address: &str, state: ParamState) -> Result<()>;
    async fn get_matching(&self, pattern: &str) -> Vec<(String, ParamState)>;
    async fn clear(&self) -> Result<()>;
}

// Built-in implementations
pub struct MemoryStorage { ... }     // Default
pub struct RedisStorage { ... }      // Optional feature
pub struct MongoStorage { ... }      // Optional feature
```

### 2. Memory Configuration

```rust
pub struct RouterConfig {
    // ... existing fields ...
    
    /// Maximum state entries (0 = unlimited)
    pub max_state_entries: usize,
    
    /// Maximum message queue per session
    pub max_queue_size: usize,
    
    /// Maximum payload size
    pub max_payload_size: usize,
    
    /// Session timeout (seconds)
    pub session_timeout: u64,
    
    /// Maximum subscriptions per session
    pub max_subscriptions_per_session: usize,
}
```

### 3. Observability

```rust
// In clasp-router
pub trait RouterMetrics: Send + Sync {
    fn on_connect(&self, session_id: &str);
    fn on_disconnect(&self, session_id: &str);
    fn on_message(&self, msg_type: &str, size: usize);
    fn on_publish(&self, address: &str, fanout: usize);
    fn on_state_update(&self, address: &str);
    fn on_error(&self, error: &str);
}

// Default: TracingMetrics (logs via tracing)
// Optional: PrometheusMetrics, OpenTelemetryMetrics
```

### 4. Node.js Server

Options:
1. **NAPI-RS** - Native Node addon, best performance
2. **WASM** - Cross-platform, slightly slower

```javascript
// Proposed API for @clasp-to/server
import { ClaspServer } from '@clasp-to/server';

const server = new ClaspServer({
  port: 7330,
  storage: new RedisStorage('redis://localhost'),
  metrics: new PrometheusMetrics(),
});

// Custom handler
server.on('message', (session, msg) => {
  console.log(`${session.name}: ${msg.address}`);
});

await server.start();
```

### 5. Crate Restructure (Long-term)

```
clasp-types        # Just types, no_std
clasp-codec        # Binary encoding, no_std  
clasp-protocol     # Full protocol, no_std optional
clasp-transport    # Network transports (feature-gated)
clasp-storage      # State backends (feature-gated)
clasp-metrics      # Observability (feature-gated)
clasp-server       # Router/server library
clasp-client       # Client library
clasp-bridges      # Protocol bridges (separate crate each?)
clasp-embedded     # Minimal no_std client+server
```

---

## Immediate Action Items

### Priority 1: Fix Deploy

1. ✅ Create standalone `deploy/relay/` with its own Cargo.toml
2. ✅ Use published crates instead of workspace paths
3. ✅ Update Dockerfile for standalone build

### Priority 2: Add Server Embedding Example

Create `examples/rust/embedded-server.rs`:

```rust
use clasp_router::{Router, RouterConfig};

#[tokio::main]
async fn main() {
    let router = Router::new(RouterConfig::default());
    
    // Your custom logic can interact with the router
    tokio::spawn(async move {
        // Example: periodically publish sensor data
        loop {
            router.publish("/sensors/cpu", get_cpu_usage()).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    
    router.serve_websocket("0.0.0.0:7330").await.unwrap();
}
```

### Priority 3: Documentation

Create `docs/architecture.md` explaining:
- What each crate does
- How they fit together
- When to use which
- Deployment options

### Priority 4: Transport Clarity for Embedded

Update `clasp-embedded` README with transport examples:
- HTTP POST
- WebSocket (ESP32 example)
- UDP
- BLE

---

## Research Topics

### Zephyr Compatibility

- [ ] Evaluate `embassy` as no_std async runtime
- [ ] Research Zephyr network stack FFI
- [ ] Consider `nrf-softdevice` for BLE on nRF chips

### State Storage Backends

- [ ] Redis (feature-gated)
- [ ] MongoDB (feature-gated)
- [ ] SQLite (feature-gated)
- [ ] Custom trait for user implementations

### Node.js Server

- [ ] NAPI-RS vs WASM benchmarks
- [ ] API design for JavaScript idioms
- [ ] Event emitter pattern vs callbacks

---

## Summary

CLASP's architecture is solid but needs:

1. **Clearer documentation** of what each crate does
2. **Standalone deploy** option using published crates
3. **Server embedding example** for custom integrations
4. **Pluggable storage** trait for state backends
5. **Better observability** hooks for production use
6. **Memory configuration** for constrained environments

The embedded story is correct (transport-agnostic) but needs better documentation and examples showing how to connect to cloud via HTTP/WebSocket.
