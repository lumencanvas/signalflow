# CLASP Project Handoff

**Last Updated:** 2026-01-16
**Current Version:** v0.1.0 (release in progress)

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

### v0.1.0 Release
**Status:** In progress (workflow running)

**Monitor:** `gh run watch --repo lumencanvas/clasp`

### Build Targets
| Platform | Target | Status |
|----------|--------|--------|
| Linux x64 | x86_64-unknown-linux-gnu | Building |
| macOS Intel | x86_64-apple-darwin | Building |
| macOS ARM | aarch64-apple-darwin | Building |
| Windows | x86_64-pc-windows-msvc | Building |
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

**Note:** v0.1.0 used default electron-builder names with versions (e.g., `CLASP.Bridge-0.1.0-arm64.dmg`). The electron-builder config has been updated to produce version-less filenames. These will take effect in v0.1.1+.

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
1. **Release v0.1.1** - Trigger a new release to get proper artifact filenames (fixed in electron-builder config)
2. **Test downloads** - Download and run on each platform after v0.1.1

### Short Term
3. **Add aarch64-linux builds** - Create Cross.toml with OpenSSL or use vendored OpenSSL
4. **Code signing** - macOS notarization, Windows Authenticode

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

### Release History
- v0.1.0 attempt 1: Failed (OpenSSL cross-compile for aarch64-linux)
- v0.1.0 attempt 2: **Success** (aarch64-linux disabled)
- v0.1.1: Pending (needed for correct artifact filenames)

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

## Contact

- **Project:** CLASP - Creative Low-Latency Application Streaming Protocol
- **Maintainer:** LumenCanvas
- **Website:** https://lumencanvas.studio
- **Issues:** https://github.com/lumencanvas/clasp/issues
