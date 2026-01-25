# Changelog

All notable changes to CLASP.

## [Unreleased]

## [3.1.0] - 2026-01-25

### Added
- MQTT server adapter: accept MQTT clients directly on the router without an external broker
- OSC server adapter: accept OSC clients via UDP with automatic session tracking
- Multi-protocol serving via `serve_all()` method
- `MultiProtocolConfig` for configuring which protocols to serve
- Rate limiting: configurable per-client message rate limits (`max_messages_per_second`)
- `try_send()` method on `TransportSender` for non-blocking sends
- `close()` method on `TransportSender` for explicit connection closing
- Relay CLI options for multi-protocol: `--mqtt-port`, `--osc-port`, `--quic-port`

### Changed
- Updated all crates to version 3.1.0
- `RouterConfig` now includes `rate_limiting_enabled` and `max_messages_per_second` fields

## [3.0.1] - 2026-01-23

### Fixed
- Fixed test compilation errors in error_tests.rs (SubscribeMessage field name)
- Fixed LWW conformance test timing for reliable state propagation
- Fixed OSC blob data test to handle rosc decoder limitations
- Fixed Python tutorial to use correct async event waiting pattern

### Documentation
- Complete documentation overhaul following Diataxis framework
- Added 70+ new documentation files covering tutorials, how-to guides, reference, and explanations
- Added persona-based navigation for different user types
- Added integration guides for TouchOSC, Resolume, QLab, Ableton, TouchDesigner, MadMapper, Home Assistant
- Added migration guides from OSC and MQTT
- Added comprehensive API reference for Rust, JavaScript, and Python

## [0.1.0] - 2024-XX-XX

### Added

#### Core
- Binary message encoding (55% smaller than JSON)
- Hierarchical address space with wildcards
- Five signal types: Param, Event, Stream, Gesture, Timeline
- Three QoS levels: Fire, Confirm, Commit
- State management with late-joiner sync
- Subscription system with pattern matching
- Atomic bundles with optional scheduling
- Lock mechanism for exclusive access

#### Transports
- WebSocket (default)
- QUIC (high-performance)
- UDP (lowest latency)
- WebRTC DataChannels (P2P)

#### Bridges
- OSC bridge
- MIDI bridge
- Art-Net bridge
- DMX bridge (serial/USB)
- MQTT bridge
- sACN bridge
- HTTP/REST bridge

#### Discovery
- mDNS advertisement and discovery
- UDP broadcast discovery

#### Security
- TLS/WSS encryption
- JWT capability tokens
- Zero-config pairing

#### Client Libraries
- Rust (clasp-client)
- JavaScript/TypeScript (@clasp-to/core)
- Python (clasp-to)

#### Embedded
- no_std Rust client (clasp-embedded)
- ESP32 support
- Minimal memory footprint (~3.6KB)

#### Tools
- CLI router (`clasp server`)
- CLI bridges (`clasp osc`, `clasp midi`, etc.)
- Desktop application

### Protocol Version
- CLASP Protocol 1.0

---

## Version Policy

CLASP follows [Semantic Versioning](https://semver.org/):

- **Major**: Breaking protocol changes
- **Minor**: New features, backwards compatible
- **Patch**: Bug fixes, no API changes

## Protocol Compatibility

| Router Version | Client Version | Compatible |
|----------------|----------------|------------|
| 0.1.x | 0.1.x | Yes |

## Migration Guides

- [Migrating from OSC](migration/from-osc.md)
- [Migrating from MQTT](migration/from-mqtt.md)

## Links

- [GitHub Releases](https://github.com/lumencanvas/clasp/releases)
- [Protocol Specification](../reference/protocol/overview.md)
