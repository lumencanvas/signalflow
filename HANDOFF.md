# CLASP Project Handoff

**Last Updated:** 2026-01-16
**Current Version:** v0.1.2 (released)

---

## Project Overview

CLASP (Creative Low-Latency Application Streaming Protocol) is a universal protocol bridge for creative applications. It connects MIDI controllers, OSC apps, DMX lights, Art-Net fixtures, MQTT sensors, and WebSocket interfaces through a unified address space.

- **Website:** https://clasp.to
- **GitHub:** https://github.com/lumencanvas/clasp
- **Maintained by:** LumenCanvas (https://lumencanvas.studio)

---

## Published Packages

| Platform | Package | Version | Registry |
|----------|---------|---------|----------|
| Rust | clasp-core | 0.1.0 | [crates.io](https://crates.io/crates/clasp-core) |
| Rust | clasp-transport | 0.1.0 | [crates.io](https://crates.io/crates/clasp-transport) |
| Rust | clasp-discovery | 0.1.0 | [crates.io](https://crates.io/crates/clasp-discovery) |
| Rust | clasp-router | 0.1.0 | [crates.io](https://crates.io/crates/clasp-router) |
| Rust | clasp-client | 0.1.0 | [crates.io](https://crates.io/crates/clasp-client) |
| Rust | clasp-bridge | 0.1.0 | [crates.io](https://crates.io/crates/clasp-bridge) |
| Rust | clasp-cli | 0.1.0 | [crates.io](https://crates.io/crates/clasp-cli) |
| JavaScript | @clasp-to/core | 0.1.0 | [npm](https://www.npmjs.com/package/@clasp-to/core) |
| Python | clasp-to | 0.1.0 | [PyPI](https://pypi.org/project/clasp-to/) |

---

## What's Implemented & Verified

### Signal Types (all 5)
| Type | QoS | Persisted | Description |
|------|-----|-----------|-------------|
| Param | Confirm | Yes | Stateful values (faders, settings) |
| Event | Confirm | No | One-shot triggers |
| Stream | Fire | No | High-rate data (30-60+ Hz) |
| Gesture | Fire | No | Phased input (start/move/end) |
| Timeline | Commit | Yes | Time-indexed automation |

### Transports
- **WebSocket** - Primary transport, port 7330, subprotocol `clasp.v2`
- **WebRTC** - DataChannels with ICE/NAT traversal
- **QUIC** - Low-latency UDP-based
- **UDP** - Raw UDP for embedded
- **BLE** - Bluetooth Low Energy with GATT services

### Protocol Bridges (8 implemented)
| Protocol | File | Status |
|----------|------|--------|
| OSC | `clasp-bridge/src/osc.rs` | Bidirectional, bundles, timestamps |
| MIDI | `clasp-bridge/src/midi.rs` | CC, notes, program change, pitchbend |
| Art-Net | `clasp-bridge/src/artnet.rs` | Multiple universes, polling |
| DMX | `clasp-bridge/src/dmx.rs` | ENTTEC Pro/Open, FTDI |
| MQTT | `clasp-bridge/src/mqtt.rs` | v3.1.1 and v5, TLS |
| WebSocket | `clasp-bridge/src/websocket.rs` | JSON bridge |
| Socket.IO | `clasp-bridge/src/socketio.rs` | Rooms, namespaces |
| HTTP | `clasp-bridge/src/http.rs` | REST API |

### Discovery
- **mDNS** - `_clasp._tcp.local` service type
- **UDP Broadcast** - Fallback on port 7331

### State Management
- Revision tracking on all params
- Conflict strategies: LWW (default), Max, Min, Lock, Merge

### Client SDKs
All have: `connect`, `set`, `get`, `emit`, `stream`, `subscribe/on`, `bundle`

- **JavaScript** - `@clasp-to/core` (npm)
- **Python** - `clasp-to` (PyPI)
- **Rust** - `clasp-client` (crates.io)

### CLI Tool
Binary: `clasp` (from clasp-cli crate)

Commands: `discover`, `get`, `set`, `watch`, `emit`, `stream`, `info`, `repl`

### Desktop App (CLASP Bridge)
Location: `apps/bridge/`

- Electron app with visual UI
- Bridge configuration
- Signal mapping with transforms (scale, invert, clamp, threshold, expression)
- Real-time monitor
- Learn mode for MIDI/OSC

---

## What's NOT Implemented

These were removed from the website after audit:

| Feature | Notes |
|---------|-------|
| **sACN/E1.31** | Empty feature flag in Cargo.toml, no implementation |
| **C Embedded SDK** | Only Rust embedded exists (`clasp-embedded`) |
| **WAN Rendezvous** | Only local discovery (mDNS + UDP broadcast) |

---

## Release Status

### v0.1.2 Release
**Status:** Released (2026-01-16)

**View:** https://github.com/lumencanvas/clasp/releases/tag/v0.1.2

### Build Targets
| Platform | Target | Status |
|----------|--------|--------|
| Linux x64 | x86_64-unknown-linux-gnu | Success |
| macOS Intel | x86_64-apple-darwin | Success |
| macOS ARM | aarch64-apple-darwin | Success |
| Windows | x86_64-pc-windows-msvc | Success |
| Linux ARM | aarch64-unknown-linux-gnu | **Disabled** (OpenSSL cross-compile issues) |

### Desktop App Artifacts
- macOS: `.dmg` (ARM and Intel)
- Windows: `.exe` installer, `.zip` portable
- Linux: `.AppImage`, `.deb`

### Download Links (in DownloadsSection.vue)
```
CLASP.Bridge-arm64.dmg     # macOS Apple Silicon
CLASP.Bridge-x64.dmg       # macOS Intel
CLASP.Bridge-Setup.exe     # Windows Installer
CLASP.Bridge-portable.exe  # Windows Portable
CLASP.Bridge.AppImage      # Linux AppImage
clasp-bridge.deb           # Linux Debian
```

**Note:** v0.1.1+ uses version-less filenames for stable download URLs.

---

## Repository Structure

```
clasp/
├── crates/                    # Rust workspace
│   ├── clasp-core/           # Types, codec, state management
│   ├── clasp-transport/      # WebSocket, QUIC, UDP, BLE, WebRTC
│   ├── clasp-discovery/      # mDNS, UDP broadcast
│   ├── clasp-router/         # Message routing, pattern matching
│   ├── clasp-client/         # High-level async client
│   ├── clasp-bridge/         # Protocol bridges
│   ├── clasp-service/        # Background service binary
│   ├── clasp-embedded/       # no_std embedded (Rust only)
│   └── clasp-cli/            # CLI tool
├── bindings/
│   ├── js/packages/clasp-core/  # @clasp-to/core
│   └── python/                   # clasp-to
├── apps/
│   └── bridge/               # Electron desktop app
├── site/                     # Vue.js website
│   └── src/components/       # Key: SpecSection, ApiSection, DownloadsSection
├── .github/workflows/
│   ├── ci.yml               # CI on push/PR
│   └── release.yml          # Release on v* tags
└── HANDOFF.md               # This file
```

---

## Key Files Reference

| Purpose | File |
|---------|------|
| Protocol spec | `site/src/components/SpecSection.vue` |
| SDK examples | `site/src/components/ApiSection.vue` |
| Download links | `site/src/components/DownloadsSection.vue` |
| Feature claims | `site/src/components/CapabilitiesSection.vue` |
| Screenshot carousel | `site/src/components/ScreenshotCarousel.vue` |
| Release workflow | `.github/workflows/release.yml` |
| CI workflow | `.github/workflows/ci.yml` |
| Electron config | `apps/bridge/package.json` |
| Core types | `crates/clasp-core/src/types.rs` |
| Frame format | `crates/clasp-core/src/frame.rs` |
| JS client | `bindings/js/packages/clasp-core/src/client.ts` |

---

## Commands

```bash
# Build all Rust crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build desktop app
cd apps/bridge && npm install && npm run build

# Run desktop app in dev
cd apps/bridge && npm run dev

# Build website
cd site && npm install && npm run build

# Serve website locally
cd site && npm run dev

# Check release status
gh run list --repo lumencanvas/clasp

# Watch release
gh run watch --repo lumencanvas/clasp

# Create new release
git tag -a vX.Y.Z -m "vX.Y.Z" && git push origin vX.Y.Z
```

---

## TODO / Next Steps

### Immediate
1. ~~**Release v0.1.1**~~ - Done (2026-01-16)
2. **Test downloads** - Download and run on each platform

### Short Term
3. **Add aarch64-linux builds** - Create Cross.toml with OpenSSL or use vendored OpenSSL
4. **Code signing** - macOS notarization, Windows Authenticode
   - **Current workaround:** Users run `xattr -cr /Applications/CLASP\ Bridge.app`
   - **To fix properly:** Need Apple Developer account ($99/yr), set up electron-builder signing:
     - `CSC_LINK` - base64 encoded .p12 certificate
     - `CSC_KEY_PASSWORD` - certificate password
     - `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID` for notarization
     - Add `@electron/notarize` package and `afterSign` hook

### Medium Term
5. **Implement sACN/E1.31** - If there's demand, create `clasp-bridge/src/sacn.rs`
6. **C bindings** - FFI wrapper around `clasp-embedded` if needed
7. **WAN discovery** - Public endpoint registration service

### Documentation
8. **API reference docs** - Generate from Rust doc comments
9. **Integration guides** - TouchOSC, Resolume, QLab, etc.
10. **Video tutorials** - Getting started, common use cases

---

## Recent Session Summary (2026-01-16)

### Completed
1. Added screenshot carousel to site (5 app screenshots)
2. Fixed Tux logo for Linux downloads
3. Rewrote spec documentation for developers
4. **Full audit of site claims vs implementation**
5. Removed false claims:
   - sACN/E1.31 (not implemented)
   - C SDK (only Rust embedded exists)
   - WAN rendezvous (not implemented)
   - ±1ms timing claim (unverified)
6. Fixed API examples:
   - Rust: `publish` → `emit`
   - JS: removed `meta` callback param
7. Fixed release workflow (removed failing aarch64-linux)
8. Retriggered v0.1.0 release
9. **v0.1.0 released successfully** - All builds passed
10. **Fixed electron-builder artifact naming** - Configured consistent filenames without version numbers
11. **Added macOS dual-architecture builds** - Both ARM and Intel DMGs will now be built
12. **Updated download URLs** - Windows portable now `.exe` instead of `.zip`
13. **v0.1.1 released successfully** - All artifacts now have version-less filenames
14. **v0.1.2 released successfully** - New risograph-style CLASP logo as app icon
15. **Redesigned site for developer appeal**:
    - Larger logo (120px → 180px)
    - New LayersSection: features + bridges + code sample (removed wire format)
    - Moved desktop app section before full spec
    - Condensed spacing throughout
16. **Added macOS Gatekeeper workaround** - Note with `xattr -cr` command on downloads page

### Release History
- v0.1.0 attempt 1: Failed (OpenSSL cross-compile for aarch64-linux)
- v0.1.0 attempt 2: **Success** (aarch64-linux disabled)
- v0.1.1: **Success** (correct version-less artifact filenames)
- v0.1.2: **Success** (new CLASP logo app icon)

---

## Wire Protocol Summary

```
Frame: 4-12 bytes header + MessagePack payload

Byte 0:     0x53 ('S' magic)
Byte 1:     Flags [QoS:2][TS:1][Enc:1][Cmp:1][Rsv:3]
Bytes 2-3:  Payload length (uint16 BE)
[Bytes 4-11: Timestamp if TS flag set]
Payload:    MessagePack message

QoS: 00=Fire, 01=Confirm, 10=Commit

Default port: 7330 (WebSocket)
Discovery port: 7331 (UDP broadcast)
Subprotocol: clasp.v2
```

---

## Holistic Analysis: The Core Problem

### Everything Exists - Just Not Connected

The CLASP project has **all the pieces**:

| Component | Quality | Status |
|-----------|---------|--------|
| Protocol design | Excellent | Spec is solid |
| clasp-core | Complete | Types, codec, state management |
| clasp-transport | Complete | WS, QUIC, UDP, BLE, WebRTC |
| clasp-router | Complete | Full protocol implementation |
| clasp-bridge | Complete | 8 protocol bridges |
| JS client | Complete | Browser-ready |
| Python client | Complete | PyPI published |
| Desktop app UI | Polished | Professional look |

**The problem is wiring:**

```
What Should Be:
  Desktop App → clasp-service → clasp-router → Real servers

What Actually Is:
  Desktop App → clasp-service → setTimeout("connected") → Nothing
```

### The Path of Least Resistance

**Immediate fix (1 day):**
```bash
# Ship clasp-router binary (already exists in tools/clasp-router)
# Desktop app spawns it instead of nothing
```

The binary is already built by the release workflow. It just needs to be:
1. Included in the desktop app bundle
2. Spawned when user creates a CLASP server

### Why This Happened

Based on commit history and code patterns:

1. **Ambitious scope** - Built comprehensive protocol, bridges, clients, discovery
2. **UI-first development** - Created beautiful desktop UI to demonstrate vision
3. **Backend deferred** - "We'll wire it up later"
4. **Integration gap** - Never closed the loop between UI and backend

This is a common pattern in creative tool development - the demo looks great, but the plumbing isn't there.

### User Experience Truth

| What User Sees | What User Thinks | What Actually Happens |
|----------------|------------------|----------------------|
| "Add CLASP Server" button | "I can create a server" | Button does nothing useful |
| Green "Connected" badge | "My server is running" | setTimeout fired |
| "Create Bridge" modal | "I can bridge protocols" | Only OSC→OSC works |
| Playground connection field | "I can test my setup" | Connection always fails |

### The 80/20 Fix

**20% of work that fixes 80% of issues:**

1. **Ship clasp-router binary** (already built)
2. **Desktop app spawns it** (few lines of code)
3. **Fix port to 7330** (find/replace)
4. **Remove fake status** (delete setTimeout)

---

## Contact

- **Project:** CLASP - Creative Low-Latency Application Streaming Protocol
- **Maintainer:** LumenCanvas
- **Website:** https://lumencanvas.studio
- **Issues:** https://github.com/lumencanvas/clasp/issues

---

# CRITICAL ANALYSIS: Desktop App Implementation Gaps

**Analysis Date:** 2026-01-16
**Analyst:** Claude (Opus 4.5)

---

## Executive Summary: Desktop App is Mostly Mocked

The CLASP Bridge desktop app has a **polished, professional UI** but **most backend functionality is not implemented**. The app shows "connected" status for servers that aren't running, and only OSC→OSC bridges actually work.

### Reality Check

| What UI Shows | What Actually Happens |
|---------------|----------------------|
| "Add CLASP Server" → "Connected" | Nothing starts. Status faked via setTimeout. |
| "Create OSC→MIDI Bridge" → "Active" | Silent failure. Only OSC→OSC works. |
| "Mappings route signals" | Transform calculated but signal never sent |
| "Scan for devices" | Works (WebSocket probing) |

---

## Desktop App: UI vs Backend Matrix

### Server Starting (ALL MOCKED)

The `start_server` IPC command is sent but **not implemented** in `clasp-service`.

| Server Type | UI Support | Backend Support | Evidence |
|-------------|------------|-----------------|----------|
| CLASP | ✅ Full form | ❌ Not implemented | `clasp-service` has no `start_server` handler |
| OSC | ✅ Full form | ❌ Not implemented | Same |
| MQTT | ✅ Full form | ❌ Not implemented | Same |
| WebSocket | ✅ Full form | ❌ Not implemented | Same |
| HTTP | ✅ Full form | ❌ Not implemented | Same |
| Art-Net | ✅ Full form | ❌ Not implemented | Same |
| DMX | ✅ Full form | ❌ Not implemented | Same |

**Code Evidence** (`electron/main.js:454-458`):
```javascript
// FAKE SUCCESS - setTimeout, not real connection
setTimeout(() => {
  server.status = 'connected';
  mainWindow?.webContents.send('device-updated', server);
}, 300);
```

### Bridge Creation

| Bridge Type | UI Support | Backend Support | Notes |
|-------------|------------|-----------------|-------|
| OSC → OSC | ✅ | ✅ Works | Only working bridge |
| OSC → CLASP | ✅ | ❌ Mocked | Returns "Unsupported source protocol" |
| MIDI → Any | ✅ | ❌ Mocked | Not implemented |
| MQTT → Any | ✅ | ❌ Mocked | Not implemented |
| Art-Net → Any | ✅ | ❌ Mocked | Not implemented |
| DMX → Any | ✅ | ❌ Mocked | Not implemented |
| WebSocket → Any | ✅ | ❌ Mocked | Not implemented |
| HTTP → Any | ✅ | ❌ Mocked | Not implemented |

**Code Evidence** (`clasp-service/main.rs:112-129`):
```rust
let mut bridge: Box<dyn Bridge> = match source.as_str() {
    "osc" => {
        // Only OSC is implemented
        Box::new(OscBridge::new(config))
    }
    _ => {
        return Err(anyhow!("Unsupported source protocol: {}", source));
    }
};
```

### clasp-service Commands

| Command | Implemented | Used By |
|---------|-------------|---------|
| `create_bridge` | ⚠️ OSC only | Bridge creation |
| `delete_bridge` | ✅ Yes | Bridge deletion |
| `list_bridges` | ✅ Yes | Startup restore |
| `send_signal` | ✅ Yes | Signal routing |
| `ping` | ✅ Yes | Health check |
| `shutdown` | ✅ Yes | App close |
| `start_server` | ❌ **NO** | Server creation |
| `stop_server` | ❌ **NO** | Server deletion |

---

## Port & Transport Confusion

### Port Numbers

| Source | Server Port | Discovery Port | Issue |
|--------|-------------|----------------|-------|
| Rust `clasp-core` | 7330 | 7331 | Canonical spec |
| JS `@clasp-to/core` | 7330 | 7331 | Matches spec |
| Site `useClasp.js` | **7331** | - | Wrong! |
| Desktop `app.js` | **7331** | - | Wrong! |

**Everyone is using 7331 but spec says 7330.**

### Transport Matrix

| Component | WebSocket | QUIC | Issue |
|-----------|-----------|------|-------|
| JS Client | ✅ | ❌ | No QUIC in browsers |
| Python Client | ✅ | ❌ | No QUIC |
| `clasp server` default | ❌ | ✅ | QUIC by default |
| `clasp server --protocol ws` | ✅ (bridge mode) | ❌ | Not full CLASP |
| `clasp-router` | ✅ (full) | ❌ | Only WS |

**Problem:** CLI defaults to QUIC, but JS/Python can only do WebSocket.

### WebSocket "Bridge Mode" Issue

When running `clasp server --protocol websocket`, it runs in bridge mode:
- Converts WS text → CLASP Set messages
- Does NOT do HELLO/WELCOME handshake
- NOT compatible with JS client expectations

For full protocol, must use `clasp-router` binary.

---

## Persona Analysis

### New Developer Journey
1. Downloads desktop app
2. Creates "CLASP Server" on 7331
3. Sees green "Connected" status
4. Opens playground, tries to connect
5. **Connection fails** (nothing actually running)
6. No error message, just confusion

### VJ/Lighting Designer Journey
1. Wants OSC from Resolume → DMX lights
2. Creates bridge in UI
3. Shows "Active"
4. Sends OSC → **Nothing happens**
5. No logs, no errors, no feedback

### Protocol Expert Journey
1. Reviews beautiful Rust crates
2. Sees all 8 bridges implemented in `clasp-bridge`
3. Tries to use via desktop app
4. Discovers only OSC works
5. Has to use CLI directly

---

## Missing UI Elements

### Server Configuration

Currently missing from server modal:

| Field | Type | Purpose |
|-------|------|---------|
| Transport | Select | WebSocket vs QUIC |
| TLS/SSL | Checkbox | Enable encryption |
| Server Name | Text | Custom display name |
| Max Connections | Number | Limit clients |
| Auth Token | Text (partial) | Only CLASP has this |
| Rate Limit | Number | Messages per second |

### General Missing

| Feature | Impact |
|---------|--------|
| Connection test button | Can't verify reachability |
| Server logs panel | Can't debug issues |
| Enable/disable toggle | Must delete to stop |
| Import/export config | Can't share setups |
| Error messages | Silent failures |

---

## Gap Priority Matrix

### P0 - Critical (Blocking Basic Usage)

| # | Gap | Current State | Fix Effort |
|---|-----|---------------|------------|
| 1 | `start_server` not implemented | Completely mocked | Medium |
| 2 | Only OSC bridges work | 7/8 protocols broken | Medium |
| 3 | Fake "connected" status | Misleading users | Low |
| 4 | Port mismatch | Connection failures | Low |
| 5 | QUIC default | JS can't connect | Low |

### P1 - High (Significantly Degrades UX)

| # | Gap | Current State | Fix Effort |
|---|-----|---------------|------------|
| 6 | No transport selection | Can't choose WS/QUIC | Low |
| 7 | Mappings don't route | Just logs to console | Medium |
| 8 | No error feedback | Silent failures | Low |
| 9 | No connection test | Can't verify setup | Low |

### P2 - Medium (Polish)

| # | Gap | Current State | Fix Effort |
|---|-----|---------------|------------|
| 10 | No server logs | Can't debug | Medium |
| 11 | No mDNS integration | Only WS probe | Medium |
| 12 | No server options | Limited config | Medium |

---

## Recommendations

### Option A: Implement Backend (Proper Fix)

1. Add `start_server`/`stop_server` to `clasp-service`:
   ```rust
   Request::StartServer { id, protocol, config } => {
       // Spawn clasp process or use clasp-bridge directly
   }
   ```

2. Implement remaining bridges in `clasp-service`:
   - Priority: MIDI, MQTT, Art-Net

3. Wire up mapping signal routing

**Effort:** ~2-3 weeks full-time

### Option B: Spawn CLI Process (Quick Fix)

Instead of implementing in clasp-service, spawn CLI:

```javascript
ipcMain.handle('start-server', async (event, config) => {
  const proc = spawn('clasp', [
    'server',
    '--protocol', config.transport || 'websocket',
    '--port', config.port || 7330
  ]);
  // Track process, forward output
});
```

**Effort:** ~2-3 days

### Option C: Honest UI (Immediate)

Show real status, don't pretend:

```javascript
// Instead of fake setTimeout success
server.status = 'not-implemented';
server.message = 'Server starting not yet available. Use CLI.';
```

**Effort:** ~2 hours

---

## Immediate Action Items

### Must Do Now

- [ ] **Fix port defaults** - Change 7331 → 7330 everywhere
- [ ] **Add transport dropdown** - Let users choose WS/QUIC
- [ ] **Remove fake status** - Show honest state
- [ ] **Add CLI instructions** - Tell users how to actually start server

### Should Do Soon

- [ ] Implement MIDI bridge in clasp-service
- [ ] Implement MQTT bridge in clasp-service
- [ ] Add connection test button
- [ ] Add error message display

### Nice to Have

- [ ] Full server management via clasp-service
- [ ] All bridges working
- [ ] Mapping signal routing
- [ ] Import/export configuration

---

## Key Files for Fixes

| File | What to Fix |
|------|-------------|
| `apps/bridge/electron/main.js:430-460` | Remove fake setTimeout, implement real |
| `apps/bridge/src/app.js:60-64` | Fix defaultAddresses ports |
| `site/src/composables/useClasp.js:16` | Fix default URL port |
| `tools/clasp-service/src/main.rs` | Add start_server command |
| `apps/bridge/src/index.html` | Add transport dropdown |

---

## Deep Dive: CLI Architecture & Why Nothing Connects

**This is the root cause of most connection issues.**

### Component Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CLASP Architecture                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐             │
│  │ clasp-core  │    │clasp-router │    │clasp-bridge │             │
│  │ (types,     │───▶│ (FULL CLASP │───▶│ (protocol   │             │
│  │  codec)     │    │  protocol)  │    │  converters)│             │
│  └─────────────┘    └──────┬──────┘    └─────────────┘             │
│                            │                                        │
│                   Binary MessagePack                                │
│                   HELLO/WELCOME/SET                                 │
│                            │                                        │
│                            ▼                                        │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    What JS Client Expects                    │   │
│  │  - WebSocket on port 7330                                    │   │
│  │  - Subprotocol: clasp.v2                                     │   │
│  │  - Binary frames (MessagePack)                               │   │
│  │  - HELLO → WELCOME handshake                                 │   │
│  │  - SET/GET/EMIT messages                                     │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  VS                                                                  │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           What `clasp server --protocol ws` Does             │   │
│  │  - Runs WebSocketBridge (bridge mode)                        │   │
│  │  - Text frames (JSON)                                        │   │
│  │  - NO HELLO/WELCOME                                          │   │
│  │  - Simple {"address": "/a", "value": 1.0} messages           │   │
│  │  - For bridging WS apps to other protocols, NOT clients      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### CLI Server Modes Explained

| Command | What Runs | Protocol | Clients Can Connect? |
|---------|-----------|----------|---------------------|
| `clasp server` | QUIC server | Full CLASP binary | ❌ No JS client support |
| `clasp server --protocol websocket` | WebSocketBridge | JSON text, no handshake | ❌ Not CLASP protocol |
| `clasp server --protocol tcp` | Echo server | Raw TCP | ❌ Not CLASP protocol |

**Evidence from `crates/clasp-cli/src/server.rs:104-120`:**
```rust
async fn run_ws_server(bind: &str, port: u16, ...) -> Result<()> {
    use clasp_bridge::{Bridge, WebSocketBridge, ...};
    // Creates WebSocketBridge - this is BRIDGE MODE
    // NOT clasp-router running WebSocket transport
}
```

### The Missing Piece: clasp-router

The **clasp-router** crate implements the full CLASP protocol:
- Binary MessagePack frames
- HELLO/WELCOME/ERROR handshake
- Proper SET/GET/EMIT/SUBSCRIBE handling
- State management with revisions

**But it's NOT exposed through the CLI.** The CLI's `server` command only runs bridges.

### Why Desktop App Can't Start Real Servers

```
Desktop App UI                    What Should Happen              What Actually Happens
────────────────                  ──────────────────              ────────────────────
"Add CLASP Server"        →       Start clasp-router        →     setTimeout("connected", 300)
"Add WebSocket Server"    →       Start clasp-router on WS  →     setTimeout("connected", 300)
"Add OSC Server"          →       Start OSC listener        →     setTimeout("connected", 300)
```

Even if clasp-service had `start_server`, it would spawn CLI which runs **bridges**, not full CLASP servers.

### The Correct Solution Path

**Option 1: Embed clasp-router in clasp-service**
```rust
// In clasp-service, when starting a CLASP server:
Request::StartServer { protocol: "clasp", port, .. } => {
    // Use clasp-router directly (it's a library crate)
    let router = clasp_router::Router::new();
    router.listen_websocket(port).await;
}
```

**Option 2: Add clasp-router binary mode to CLI**
```rust
// New CLI command:
#[derive(Subcommand)]
enum Commands {
    Server { ... },  // Existing bridge-mode
    Router {         // NEW: Full CLASP protocol
        #[arg(short, long, default_value = "7330")]
        port: u16,
        #[arg(short, long, default_value = "websocket")]
        transport: String,
    },
}
```

**Option 3: Separate clasp-router binary**
```bash
# Build and ship as separate binary
cargo build --release -p clasp-router --features bin
# Desktop app spawns this instead of CLI
```

### Why QUIC as Default is Wrong

| User Type | Primary Platform | QUIC Support |
|-----------|-----------------|--------------|
| Web developer | Browser | ❌ No |
| TouchDesigner/Resolume | Desktop | ❌ No native |
| Arduino/ESP32 | Embedded | ❌ No |
| Unity/Unreal | Game engines | ❌ No native |

**QUIC is only usable by native Rust applications.** The only reason to choose QUIC is for its low-latency properties in controlled native environments - a niche use case.

**Recommendation:** Default should be WebSocket (port 7330) for maximum compatibility.

---

## Comprehensive Fix Strategy

### Phase 1: Immediate Honesty (Day 1)

1. **Remove fake status** - Show "not implemented" instead of "connected"
2. **Add CLI instructions** - Tell users how to actually run servers
3. **Fix port defaults** - 7330 everywhere, not 7331

### Phase 2: Quick Wins (Week 1)

1. **Add transport dropdown** - Let users choose WebSocket/QUIC
2. **Add `clasp router` command** - Expose clasp-router via CLI
3. **Update playground** - Connect to proper clasp-router

### Phase 3: Full Implementation (Month 1)

1. **Embed clasp-router in clasp-service** - Real server starting
2. **Implement all bridges in clasp-service** - Not just OSC
3. **Wire up signal routing** - Mappings actually work
4. **Add mDNS integration** - Proper discovery

---

## Verification Tests

To verify fixes work:

1. **Server Starting:**
   ```bash
   # In desktop app, create CLASP server on 7330
   # Then verify:
   lsof -i :7330  # Should show listening process
   ```

2. **Playground Connection:**
   ```bash
   # After creating server in desktop app
   # Open playground, connect to ws://localhost:7330
   # Should get WELCOME message
   ```

3. **Bridge Working:**
   ```bash
   # Create OSC→MIDI bridge
   # Send OSC: /test 0.5
   # Verify MIDI CC output
   ```
