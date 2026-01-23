# CLASP Project Master Plan

**Last Updated:** 2026-01-23
**Version:** 0.1.8
**Status:** Active Development

---

## 🔍 Latest Session Summary (2026-01-22/23)

**Changes Made:**
1. **Desktop App UI Improvements:**
   - Sidebar width increased to 320px (was 220px)
   - Window size reduced by 10% (1152×810)
   - Remote router support: click discovered servers to add as connection targets
   - Remote routers display with "REMOTE" badge and blue styling
   - Remote routers appear in protocol connection dropdowns

2. **Bridge Service:**
   - Built `clasp-service` binary (was missing, causing "Bridge service not ready" errors)
   - Improved error logging in `startBridgeService()` function
   - Binary path verification added

3. **Documentation:**
   - All public docs updated with protocol-centric terminology
   - Handoff document updated with completion status

**Codebase Health Check (2026-01-23):**
- ✅ All Rust tests passing (15+ tests)
- ✅ No compilation errors
- ✅ Minor warnings only (unused imports, dead code - normal dev warnings)
- ✅ Desktop app terminology consistent (ADD ROUTER, ADD PROTOCOL, ADD OUTPUT)
- ✅ Documentation consistent with codebase
- ✅ State management properly organized
- ✅ Binary build scripts correct

---

## 🎯 For New LLM: START HERE

**Read `.internal/HANDOFF-GUIDE.md` first** - It explains where to start, what's been done, and what needs to be done.

**Quick Start:**
1. Read `.internal/COMPLETE-ARCHITECTURE-SUMMARY.md` (5 min) - The definitive model
2. Read `.internal/MASTER-CONSOLIDATION-PLAN.md` (15 min) - File-by-file updates
3. Read `.internal/IMPLEMENTATION-ROADMAP.md` (10 min) - Phase-by-phase plan
4. Start Phase 1: Fix "internal" router connection

---

## Quick Context

CLASP (Creative Low-Latency Application Streaming Protocol) is a universal protocol bridge for creative applications. This document consolidates all internal planning.

**Version Context:** Protocol specification is version 1.0 (references to v3 removed). Package versions (0.1.x) will continue to increment independently. The protocol itself is stable at v1.

**Architecture Model:** Protocol-Centric Organization (see `.internal/COMPLETE-ARCHITECTURE-SUMMARY.md` for definitive model)

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
- [x] Documentation updated for protocol-centric terminology (2026-01-22)
- [x] "Internal" router connection implemented in Electron app
- [x] clasp-service binary built and integrated (2026-01-22)
- [x] Desktop app sidebar widened, window resized (2026-01-22)
- [x] Remote router support: click discovered servers to add as connection target (2026-01-22)

### 🔄 In Progress

- [ ] Server-in-application examples (Rust, Node.js, Python)
- [ ] Version coordination across packages
- [x] Desktop app protocol updates (clasp-service built and integrated)
- [x] Desktop app server scanning improvements
  - [x] Click discovered server to add as remote router
  - [x] Remote routers display with REMOTE badge
  - [x] Remote routers appear in protocol connection dropdowns
  - [x] Sidebar made wider (320px)
  - [x] Window size reduced 10%
- [ ] Bridge and server setup documentation with examples
- [ ] Protocol mapping examples (X→CLASP and CLASP→X)

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

**Status:** Needs protocol update verification + UI/UX improvements for clarity

**⚠️ CRITICAL ARCHITECTURE ISSUES FOUND:**

See `.internal/ARCHITECTURE-FINDINGS-AND-RECOMMENDATIONS.md` for complete analysis.

**Key Findings:**
1. **"Internal" router connection is NOT implemented** - Protocol servers create bridges with `target_addr: 'internal'`, but signals are NOT automatically forwarded to CLASP routers. They just appear in the signal monitor.
2. **Bridges are hidden** - Protocol servers create bridges internally, but they're not added to `state.bridges` array, so users can't see/edit them.
3. **Terminology is misleading** - "ADD SERVER" suggests standalone servers, but they also create hidden bridges.
4. **Protocol adapters are bidirectional** - Most protocols (OSC, MIDI, MQTT, WebSocket, HTTP, Art-Net, sACN) support bidirectional communication. Only DMX is output-only.
5. **Transports are router settings** - WebSocket, QUIC, TCP are settings on the CLASP router, not separate components.

**See `.internal/COMPONENT-CAPABILITIES-MAP.md` for complete analysis of what each component does and can do.**

**Critical Tasks:**
- [ ] **FIX "INTERNAL" ROUTER CONNECTION (CRITICAL):**
  - [ ] Implement actual forwarding of bridge signals to CLASP router
  - [ ] When `target_addr: 'internal'`, find first running CLASP router and connect
  - [ ] Forward all bridge signals to router via WebSocket
  - [ ] Show connection status in UI (which router it connects to)
  - [ ] Error if no CLASP router exists when trying to connect
- [ ] **SHOW AUTO-CREATED BRIDGES (HIGH PRIORITY):**
  - [ ] Add auto-created bridges to `state.bridges` array
  - [ ] Mark as `autoCreated: true` and link to server
  - [ ] Show in Bridges tab with "Auto" label
  - [ ] Allow user to edit/delete auto-created bridges
- [ ] **REORGANIZE UI INTO CLEAR SECTIONS (HIGH PRIORITY):**
  - [ ] **CLASP ROUTERS** section - for creating routers
  - [ ] **PROTOCOL CONNECTIONS** section - organize by protocol (not by role)
    - [ ] Button: "ADD PROTOCOL" (not "ADD SERVER")
    - [ ] Modal: Select protocol first, then configure role (server/client/device) as a setting
    - [ ] Show connection status: "→ Connected to: CLASP Router"
    - [ ] Allow multiple connections per protocol (e.g., OSC on port 9000 and 8000)
  - [ ] **DIRECT CONNECTIONS** section - protocol-to-protocol bridges (bypass CLASP)
    - [ ] Button: "CREATE DIRECT BRIDGE" or "CREATE PROTOCOL-TO-PROTOCOL BRIDGE"
    - [ ] Make it clear these bypass CLASP router
  - [ ] Update terminology: "CLASP Server" → "CLASP Router", "OSC Server" → "OSC Connection"
  - [ ] See `.internal/DEEP-UI-ARCHITECTURE-ANALYSIS.md` for detailed rationale
- [ ] **MAKE ADAPTER CONNECTION EXPLICIT (HIGH PRIORITY):**
  - [ ] Add dropdown in adapter modal to select CLASP router
  - [ ] Show which router each adapter connects to in adapter list
  - [ ] Error if no router exists when trying to connect
- [ ] **ADD TRANSPORT SETTINGS TO ROUTER (MEDIUM PRIORITY):**
  - [ ] Add transport selection to router creation/editing modal
  - [ ] Checkboxes: WebSocket, QUIC, TCP
  - [ ] Show active transports in router list
- [ ] Verify WebSocket connects with `clasp` subprotocol
- [ ] Test message encoding/decoding with binary format
- [ ] Update any UI that references v2/v3
- [ ] Test with latest clasp-router
- [ ] **IMPROVE SERVER SCANNING UI:**
  - [ ] Click discovered server to add it (with rename option)
  - [ ] Option to create bridge to discovered server
  - [ ] Better visual feedback during scan
  - [ ] Show server capabilities/metadata in list
  - [ ] Persist discovered servers with custom names
- [ ] **CLARIFY BRIDGE VS SERVER IN UI (HIGH PRIORITY):**
  - [ ] See detailed plans:
    - [ ] `.internal/ARCHITECTURE-FINDINGS-AND-RECOMMENDATIONS.md` - Complete analysis with recommendations
    - [ ] `.internal/ACTUAL-ARCHITECTURE-MAP.md` - What each component actually does
    - [ ] `.internal/CLEAR-ARCHITECTURE-PROPOSAL.md` - Recommended path forward
    - [ ] `.internal/COMPONENT-CAPABILITIES-MAP.md` - What each component does and can do
    - [ ] `.internal/PROTOCOL-ADAPTER-ROLES.md` - Server vs Client roles for adapters
    - [ ] `.internal/UI-UX-IMPROVEMENTS.md` - Proposed UI changes
  - [ ] **Key Finding:** Servers (`state.servers`) and Bridges (`state.bridges`) are separate
    - [ ] Servers auto-create bridges to CLASP internally, but don't show in Bridges tab
    - [ ] This is why users are confused - they add a "server" but don't see it as a bridge
  - [ ] **Recommended Changes:**
    - [ ] Rename "MY SERVERS" → "CONNECTED PROTOCOLS" or "PROTOCOL CONNECTIONS"
    - [ ] Rename "ADD SERVER" → "CONNECT PROTOCOL"
    - [ ] Update modal title to "CONNECT PROTOCOL"
    - [ ] Add description: "Creates a bridge that connects [protocol] to CLASP router"
    - [ ] Show "OSC Bridge → CLASP Router" in list instead of "OSC Server"
    - [ ] Add note: "This bridge won't appear in 'Protocol Bridges' tab (it's auto-managed)"
    - [ ] Add visual indicators showing connection to CLASP router
    - [ ] Use language accessible to non-technical digital artists
    - [ ] Maintain design consistency with current style
- [ ] **CLARIFY BRIDGE VS SERVER TERMINOLOGY IN UI:**
  - [ ] **Modal Title & Description:**
    - [ ] Change "ADD SERVER" modal title to "ADD PROTOCOL BRIDGE" or "CONNECT PROTOCOL"
    - [ ] Add clear description: "Connect [protocol] devices to CLASP. Messages are automatically translated and routed through CLASP."
    - [ ] Show visual indicator that bridge connects to CLASP router (e.g., "→ CLASP Router" badge)
  - [ ] **Button Labels:**
    - [ ] Change "START SERVER" to "START BRIDGE" or "CONNECT"
    - [ ] Update sidebar button from "+ ADD SERVER" to "+ ADD BRIDGE" or "+ CONNECT PROTOCOL"
  - [ ] **Server List Display:**
    - [ ] Show connection status: "Connected to CLASP Router" or "Bridge to CLASP"
    - [ ] Add icon/badge indicating it's a bridge (not standalone server)
    - [ ] Show which CLASP router it's connected to (if multiple routers exist)
  - [ ] **Help Text & Tooltips:**
    - [ ] Add tooltip to "ADD SERVER" button: "Create a bridge that connects [protocol] devices to CLASP"
    - [ ] Add inline help in modal: "This creates a bridge that translates [protocol] messages to CLASP format and routes them through the CLASP router."
    - [ ] Clarify: "Your [protocol] devices can connect here, and their messages will be available to all CLASP clients."
  - [ ] **Visual Design:**
    - [ ] Add visual flow indicator in modal: "[Protocol] Device → Bridge → CLASP Router → Other Clients"
    - [ ] Use consistent terminology: "Bridge" not "Server" for protocol connections
    - [ ] Keep "Server" only for CLASP native protocol server
  - [ ] **For Digital Artists (Non-Technical Users):**
    - [ ] Use simple language: "Connect your [protocol] gear" instead of "Start [protocol] server"
    - [ ] Explain benefit: "Makes your [protocol] devices work with CLASP"
    - [ ] Show example: "TouchOSC → OSC Bridge → Works with all CLASP apps"
    - [ ] Avoid technical jargon in UI labels
  - [ ] **Consistency:**
    - [ ] Ensure all protocol options use same terminology
    - [ ] Update all help text to be consistent
    - [ ] Make sure "Protocol Bridges" tab matches sidebar terminology
  - [ ] **Future: Standalone Servers:**
    - [ ] Plan separate section/option for standalone protocol servers
    - [ ] Label clearly: "Standalone [Protocol] Server (No CLASP)"
    - [ ] Explain when to use: "For apps that only speak [protocol], no CLASP translation"

**Files to check:**
- `apps/bridge/electron/main.js` ✅ (updated to `clasp`)
- `apps/bridge/src/app.js` (needs server scanning improvements)
- `apps/bridge/src/index.html` (UI for discovered servers)
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

### Version Context

**Protocol Specification:** Version 1.0 (references to v3 removed from spec)
**Package Versions:** Continue to increment independently (0.1.8, 0.1.9, etc.)
**Code References:** May still reference "v3" internally (binary encoding version), but protocol spec is v1

**Important:** The protocol specification document (CLASP-Protocol.md) is version 1.0. Internal code comments may reference "v3" to indicate the binary encoding format, but the official protocol version is 1.

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

**See `.internal/MASTER-CONSOLIDATION-PLAN.md` for comprehensive file-by-file update plan.**

### Critical Documentation Updates ✅ COMPLETED (2026-01-22)

- [x] **README.md**
  - [x] Update Quick Start: "protocol connections" terminology
  - [x] Clarify: Bridge commands create protocol connections to router
  - [x] Update terminology: "CLASP Server" → "CLASP Router"
  
- [x] **docs/index.md**
  - [x] Update to protocol-centric model
  - [x] Update terminology throughout

- [x] **docs/guides/bridge-setup.md**
  - [x] Update Desktop App section: "ADD SERVER" → "ADD PROTOCOL"
  - [x] Clarify protocol connections vs direct bridges
  - [x] Update all terminology

- [x] **docs/guides/desktop-app-servers.md**
  - [x] Rewritten for protocol-centric model
  - [x] Title: "Desktop App: Understanding Protocol Connections"
  - [x] Updated all terminology

- [x] **crates/clasp-cli/README.md**
  - [x] Update "Start Protocol Bridges" → "Start Protocol Connections"
  - [x] Clarify connection to router
  - [x] Update examples

- [x] **docs/protocols/README.md**
  - [x] Update terminology throughout

### Remaining Documentation (Lower Priority)

- [ ] **docs/guides/protocol-mapping.md**
  - [ ] Verify terminology
  - [ ] Add note about bidirectional connections
  - [ ] Clarify protocol-to-protocol vs protocol-to-CLASP

- [ ] **docs/architecture.md**
  - [ ] Update diagrams for protocol-centric model
  - [ ] Clarify protocol connections vs direct bridges

- [ ] **docs/protocols/*.md** (individual protocol docs)
  - [ ] Review each protocol doc
  - [ ] Ensure consistent terminology
  - [ ] Add connection examples

- [ ] **Website (site/)**
  - [ ] Review all pages
  - [ ] Update terminology throughout
  - [ ] Update examples
  - [ ] Update screenshots (after UI changes)

### Additional Documentation

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

## Related Internal Documents

### 🎯 Start Here
- **`.internal/COMPLETE-ARCHITECTURE-SUMMARY.md`** - **DEFINITIVE MODEL** - Complete summary of protocol-centric architecture
- **`.internal/MASTER-CONSOLIDATION-PLAN.md`** - **COMPREHENSIVE UPDATE PLAN** - File-by-file updates for all docs, README, website, app
- **`.internal/IMPLEMENTATION-ROADMAP.md`** - Phase-by-phase implementation plan with timelines
- **`.internal/QUICK-REFERENCE.md`** - Quick reference for developers

### Architecture & Analysis
- `.internal/DEEP-UI-ARCHITECTURE-ANALYSIS.md` - Deep UI/UX analysis with protocol-centric recommendations
- `.internal/COMPONENT-CAPABILITIES-MAP.md` - What each component does and can do
- `.internal/PROTOCOL-ADAPTER-ROLES.md` - Server vs Client roles for adapters
- `.internal/ARCHITECTURE-FINDINGS-AND-RECOMMENDATIONS.md` - Complete architecture analysis
- `.internal/ACTUAL-ARCHITECTURE-MAP.md` - What each component actually does
- `.internal/CLEAR-ARCHITECTURE-PROPOSAL.md` - Recommended path forward
- `.internal/UI-UX-IMPROVEMENTS.md` - Proposed UI changes

### Implementation Status
- ✅ All architecture analysis complete
- ✅ Protocol-centric model determined and documented
- ✅ Master consolidation plan created (file-by-file updates)
- ✅ Implementation roadmap created (phase-by-phase)
- ✅ Complete architecture summary created
- 🚀 Ready for implementation

---

*This document should be updated with each significant change.*
