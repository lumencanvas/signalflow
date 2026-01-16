# CLASP Project Handoff

**Creative Low-Latency Application Streaming Protocol**

This document outlines the planned work to expand CLASP - a full-featured protocol bridging and API gateway platform.

---

## Phase 1: Rebrand to CLASP (COMPLETED)

### 1.1 Update Project Identity
- [x] Replace all "SignalFlow" references with "CLASP"
- [x] Replace all "signalflow" in code with "clasp"
- [x] Update package names:
  - `signalflow-core` → `clasp-core`
  - `signalflow-transport` → `clasp-transport`
  - `signalflow-discovery` → `clasp-discovery`
  - `signalflow-bridge` → `clasp-bridge`
  - `signalflow-router` → `clasp-router`
  - `signalflow-client` → `clasp-client`
  - `signalflow-embedded` → `clasp-embedded`
  - `signalflow-wasm` → `clasp-wasm`
- [x] Update Cargo.toml workspace and all crate Cargo.toml files
- [x] Update package.json files
- [x] Update repository references

### 1.2 Logo & Branding Assets
- [ ] Save new CLASP logo to `assets/logo.svg`
- [ ] Create favicon variants (16x16, 32x32, 64x64, 180x180)
- [ ] Create app icons for Electron (icns, ico, png)
- [ ] Update titlebar icon in app
- [ ] Create social preview image for GitHub
- [ ] Update color scheme if needed (current: teal accent on paper brutalist)

### 1.3 Update App UI
- [ ] Replace "SIGNALFLOW BRIDGE" titlebar text with "CLASP"
- [ ] Update version badge
- [ ] Update status bar protocol text
- [ ] Update any references to "SignalFlow Protocol v2"

### Files Updated:
```
Cargo.toml - DONE
crates/*/Cargo.toml - DONE
apps/bridge/package.json - DONE
apps/bridge/electron/preload.js - DONE
tools/clasp-service/ - DONE
tools/clasp-cli/ - DONE (renamed from sf-cli)
tools/clasp-router/ - DONE (renamed from sf-router)
tools/clasp-test/ - DONE (renamed from sf-test)
README.md - DONE
bindings/python/ - DONE (renamed module to clasp)
bindings/js/packages/clasp-core/ - DONE (renamed from signalflow-core)
```

---

## Phase 2: New Protocol Support

### 2.1 MQTT Bridge
**Rust Crate: `clasp-bridge/src/mqtt.rs`**

- [ ] Add `rumqttc` or `paho-mqtt` to dependencies
- [ ] Implement `MqttBridge` struct
- [ ] Support MQTT 3.1.1 and 5.0
- [ ] Configuration:
  ```rust
  struct MqttBridgeConfig {
      broker_url: String,        // mqtt://localhost:1883
      client_id: String,
      username: Option<String>,
      password: Option<String>,
      topics: Vec<String>,       // Subscribe topics
      qos: u8,                   // 0, 1, or 2
      clean_session: bool,
      keep_alive_secs: u16,
      tls: Option<TlsConfig>,
  }
  ```
- [ ] Implement topic-to-address mapping (MQTT topic → CLASP address)
- [ ] Support wildcards: `+` (single level) and `#` (multi-level)
- [ ] Bidirectional: Subscribe and Publish
- [ ] Handle retained messages
- [ ] Handle Last Will and Testament (LWT)

**Frontend Updates:**
- [ ] Add MQTT to protocol dropdowns
- [ ] Add MQTT-specific fields in bridge/mapping modals:
  - Broker URL
  - Client ID
  - Username/Password (optional)
  - Topic pattern
  - QoS level
  - TLS toggle

### 2.2 WebSocket Bridge
**Rust Crate: `clasp-bridge/src/websocket.rs`**

- [ ] Use existing `tokio-tungstenite` dependency
- [ ] Implement `WebSocketBridge` struct
- [ ] Support both client and server modes
- [ ] Configuration:
  ```rust
  struct WebSocketBridgeConfig {
      mode: WsMode,              // Client or Server
      url: String,               // ws://localhost:8080 or bind address
      path: Option<String>,      // /ws for server mode
      subprotocols: Vec<String>,
      headers: HashMap<String, String>,
      ping_interval: Option<Duration>,
      message_format: WsMessageFormat,  // JSON, MsgPack, Raw
  }
  ```
- [ ] Auto-reconnect for client mode
- [ ] Multiple client support for server mode
- [ ] Binary and text message support
- [ ] Heartbeat/ping-pong handling

**Frontend Updates:**
- [ ] Add WebSocket to protocol dropdowns
- [ ] Add WS-specific fields:
  - Mode (Client/Server)
  - URL or bind address
  - Path (for server)
  - Message format (JSON/MsgPack/Raw)
  - Headers (key-value pairs)

### 2.3 Socket.IO Bridge
**Rust Crate: `clasp-bridge/src/socketio.rs`**

- [ ] Add `rust-socketio` or `socketio-rs` to dependencies
- [ ] Implement `SocketIOBridge` struct
- [ ] Support Socket.IO v4 protocol
- [ ] Configuration:
  ```rust
  struct SocketIOBridgeConfig {
      url: String,
      namespace: String,         // Default: "/"
      events: Vec<String>,       // Events to listen for
      auth: Option<serde_json::Value>,
      reconnect: bool,
      transports: Vec<Transport>, // WebSocket, Polling
  }
  ```
- [ ] Event-to-address mapping (Socket.IO event → CLASP address)
- [ ] Support acknowledgments
- [ ] Support rooms (for server mode)
- [ ] Binary event support

**Frontend Updates:**
- [ ] Add Socket.IO to protocol dropdowns
- [ ] Add Socket.IO-specific fields:
  - Server URL
  - Namespace
  - Event names (comma-separated or list)
  - Auth payload (JSON editor)

### 2.4 HTTP/REST Bridge
**Rust Crate: `clasp-bridge/src/http.rs`**

This is the most complex addition - a full REST API server/client.

#### 2.4.1 REST Server Mode
- [ ] Add `axum` or `actix-web` to dependencies
- [ ] Implement `HttpServerBridge` struct
- [ ] Configuration:
  ```rust
  struct HttpServerConfig {
      bind_addr: String,         // 0.0.0.0:3000
      endpoints: Vec<EndpointConfig>,
      cors: Option<CorsConfig>,
      auth: Option<AuthConfig>,
      rate_limit: Option<RateLimitConfig>,
  }

  struct EndpointConfig {
      path: String,              // /api/lights/:id
      method: HttpMethod,        // GET, POST, PUT, DELETE, PATCH
      params: Vec<ParamConfig>,  // Path params, query params
      body_schema: Option<JsonSchema>,
      response_schema: Option<JsonSchema>,
      clasp_address: String,     // Target CLASP address to trigger
      transform: TransformConfig,
  }

  struct ParamConfig {
      name: String,
      location: ParamLocation,   // Path, Query, Header, Body
      required: bool,
      default: Option<serde_json::Value>,
      mapping: String,           // How to map to CLASP message
  }
  ```
- [ ] Dynamic endpoint registration
- [ ] Path parameter extraction (`:id`, `:name`)
- [ ] Query parameter handling
- [ ] Request body parsing (JSON, form-data, raw)
- [ ] Response generation from CLASP messages
- [ ] Request/response logging
- [ ] OpenAPI spec generation

#### 2.4.2 REST Client Mode
- [ ] Implement `HttpClientBridge` struct
- [ ] Configuration:
  ```rust
  struct HttpClientConfig {
      base_url: String,
      endpoints: Vec<ClientEndpointConfig>,
      auth: Option<ClientAuthConfig>,
      timeout: Duration,
      retry: Option<RetryConfig>,
  }

  struct ClientEndpointConfig {
      trigger_address: String,   // CLASP address that triggers this
      method: HttpMethod,
      path: String,
      headers: HashMap<String, String>,
      body_template: Option<String>,  // Handlebars/template
      response_mapping: String,  // How to map response to CLASP
  }
  ```
- [ ] Template-based request building
- [ ] Response parsing and mapping
- [ ] Authentication (Basic, Bearer, API Key, OAuth2)
- [ ] Retry with backoff
- [ ] Request queuing

#### 2.4.3 REST API Designer UI
This needs a dedicated UI section in the app:

- [ ] New "API" tab in main interface
- [ ] Endpoint list view
- [ ] Endpoint editor:
  - Method selector (GET/POST/PUT/DELETE/PATCH)
  - Path input with param highlighting
  - Param configuration table
  - Request body schema editor (JSON)
  - Response schema editor (JSON)
  - Test button to send sample request
  - Map to CLASP address selector
  - Transform configuration
- [ ] API documentation preview
- [ ] Export OpenAPI spec button
- [ ] Import from OpenAPI spec
- [ ] cURL command generator

**Frontend Files to Create:**
```
apps/bridge/src/components/api-designer.js
apps/bridge/src/styles/api-designer.css
```

---

## Phase 3: Enhanced Mapping System

### 3.1 Advanced Transform Functions
- [ ] **Expression Engine**: Allow JavaScript-like expressions
  ```
  value * 2 + 10
  Math.round(value)
  value > 0.5 ? 1 : 0
  ```
- [ ] **Lookup Tables**: Map discrete values
  ```
  { 0: "off", 1: "low", 2: "medium", 3: "high" }
  ```
- [ ] **Curve Functions**: Non-linear transforms
  - Ease-in, ease-out, ease-in-out
  - Exponential, logarithmic
  - Custom bezier curves
- [ ] **Aggregation**: Combine multiple values
  - Average, sum, min, max
  - Moving average
  - Rate of change

### 3.2 Conditional Routing
- [ ] Route based on value conditions
- [ ] Route based on source metadata
- [ ] Multi-target routing (one source → multiple targets)
- [ ] Priority-based routing

### 3.3 Message Construction
- [ ] Build complex messages from multiple sources
- [ ] Template-based message formatting
- [ ] JSON path extraction and injection
- [ ] Array/object manipulation

### 3.4 Payload Field Mapping
For protocols with structured payloads:
- [ ] Map specific JSON fields to/from CLASP values
- [ ] Support nested paths: `data.sensors[0].temperature`
- [ ] Array element selection
- [ ] Default values for missing fields

---

## Phase 4: Protocol Improvements

### 4.1 CLASP Protocol Specification
- [ ] Define formal message format specification
- [ ] Document address naming conventions
- [ ] Define standard message types
- [ ] Version negotiation protocol
- [ ] Security/authentication layer spec

### 4.2 Core Library Enhancements
- [ ] Add HTTP transport option
- [ ] Add QUIC transport option (already have quinn)
- [ ] Message batching for high-throughput
- [ ] Message compression
- [ ] Message encryption
- [ ] Request-response patterns
- [ ] Streaming patterns

---

## Phase 5: Documentation Site

### 5.1 Create Documentation Structure
```
docs/
├── index.md                 # Overview
├── getting-started/
│   ├── installation.md
│   ├── quick-start.md
│   └── first-bridge.md
├── concepts/
│   ├── protocol.md          # CLASP protocol spec
│   ├── addresses.md         # Address naming
│   ├── messages.md          # Message formats
│   ├── bridges.md           # Bridge concepts
│   └── mappings.md          # Mapping concepts
├── protocols/
│   ├── osc.md
│   ├── midi.md
│   ├── dmx.md
│   ├── artnet.md
│   ├── mqtt.md
│   ├── websocket.md
│   ├── socketio.md
│   └── http-rest.md
├── app/
│   ├── overview.md
│   ├── bridges-tab.md
│   ├── mappings-tab.md
│   ├── api-designer.md
│   └── monitor.md
├── api/
│   ├── rust-sdk.md
│   ├── javascript-sdk.md
│   └── rest-api.md
├── examples/
│   ├── osc-to-midi.md
│   ├── mqtt-to-dmx.md
│   ├── rest-api-gateway.md
│   └── home-automation.md
└── reference/
    ├── config.md
    ├── transforms.md
    └── cli.md
```

### 5.2 Documentation Site Setup
- [ ] Use VitePress, Docusaurus, or Astro Starlight
- [ ] Configure site with CLASP branding
- [ ] Add search functionality
- [ ] Add API reference generation
- [ ] GitHub Pages deployment

### 5.3 Write Core Documentation
- [ ] Protocol specification document
- [ ] Getting started guide
- [ ] Each protocol's bridge documentation
- [ ] REST API designer tutorial
- [ ] Example walkthroughs

---

## Phase 6: Service & CLI Improvements

### 6.1 Unified CLASP Service
Rename `sf-bridge-service` to `clasp-service`:
- [ ] Rename tool directory
- [ ] Add all new protocol handlers
- [ ] Add REST server capability
- [ ] Add configuration file support (YAML/TOML)
- [ ] Add hot-reload for config changes
- [ ] Add metrics/stats endpoint

### 6.2 CLI Tool
Create `clasp-cli`:
- [ ] `clasp init` - Initialize project config
- [ ] `clasp serve` - Start the service
- [ ] `clasp bridge create` - Create bridge from CLI
- [ ] `clasp mapping create` - Create mapping from CLI
- [ ] `clasp send` - Send test message
- [ ] `clasp monitor` - Terminal-based signal monitor
- [ ] `clasp docs` - Generate API docs

---

## Implementation Priority Order

### Sprint 1: Foundation (Week 1-2)
1. Rebrand to CLASP (names, logos, UI)
2. WebSocket bridge (simplest new protocol)
3. Basic docs site structure

### Sprint 2: Real-time Protocols (Week 3-4)
1. MQTT bridge
2. Socket.IO bridge
3. Enhanced mapping transforms

### Sprint 3: REST Gateway (Week 5-7)
1. HTTP REST server mode
2. HTTP REST client mode
3. API Designer UI
4. OpenAPI generation

### Sprint 4: Polish (Week 8)
1. Documentation completion
2. CLI tool
3. Testing & bug fixes
4. Example projects

---

## Technical Decisions Needed

1. **HTTP Framework**: Axum vs Actix-web vs Warp?
   - Recommendation: Axum (Tokio ecosystem, good ergonomics)

2. **MQTT Client**: rumqttc vs paho-mqtt?
   - Recommendation: rumqttc (pure Rust, async)

3. **Socket.IO**: rust-socketio vs custom implementation?
   - Recommendation: rust-socketio (maintained, complete)

4. **Docs Framework**: VitePress vs Docusaurus vs Starlight?
   - Recommendation: VitePress (lightweight, Vue-based)

5. **Config Format**: YAML vs TOML vs JSON?
   - Recommendation: TOML for config files, JSON for API

---

## File Changes Summary

### New Files to Create
```
assets/logo.svg
assets/favicon.ico
assets/icon.icns
assets/icon.ico

crates/clasp-bridge/src/mqtt.rs
crates/clasp-bridge/src/websocket.rs
crates/clasp-bridge/src/socketio.rs
crates/clasp-bridge/src/http/mod.rs
crates/clasp-bridge/src/http/server.rs
crates/clasp-bridge/src/http/client.rs

apps/bridge/src/components/api-designer.js
apps/bridge/src/styles/api-designer.css

tools/clasp-service/ (rename from sf-bridge-service)
tools/clasp-cli/

docs/ (entire directory)
```

### Files to Rename/Update
```
All Cargo.toml files (workspace + crates)
All package.json files
All source files with "signalflow" references
README.md
LICENSE files (update project name)
```

---

## Current State

The project currently has:
- ✅ Core protocol library (Rust)
- ✅ OSC, MIDI, Art-Net, DMX bridges
- ✅ MQTT, WebSocket, Socket.IO bridges
- ✅ HTTP/REST bridge
- ✅ Electron desktop app with Paper Brutalist UI
- ✅ Tabbed interface (Bridges, Mappings, Monitor)
- ✅ Basic mapping system with transforms
- ✅ Learn mode for address capture
- ✅ Rust bridge service (JSON-RPC via stdin/stdout)
- ✅ Documentation website at clasp.to
- ✅ Complete rebrand from SignalFlow to CLASP
- ✅ Python bindings (clasp-to)
- ✅ JavaScript/TypeScript bindings (@clasp-to/core)

Next steps:
- [ ] Publish to crates.io
- [ ] Publish to npm
- [ ] Publish to PyPI
- [ ] Create GitHub release with binaries

---

*Last Updated: 2026-01-15*
*Project: CLASP - Creative Low-Latency Application Streaming Protocol*
*Website: https://clasp.to*
