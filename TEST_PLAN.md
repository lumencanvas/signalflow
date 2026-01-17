# CLASP Comprehensive Test Plan

**Created:** 2026-01-16
**Last Updated:** 2026-01-16
**Goal:** 100% coverage of all CLASP protocol functionality
**Status:** IN PROGRESS

## Quick Start (For New Session)

```bash
# 1. Verify current tests pass
cargo run -p clasp-test-suite --bin run-all-tests

# 2. See what needs to be done
grep "❌" TEST_PLAN.md

# 3. Start with highest priority (Phase 4: Client Library)
# Create: test-suite/src/bin/client_tests.rs
```

---

## Overview

This document tracks the comprehensive test suite development for CLASP. The goal is to prove the protocol can do everything it claims through exhaustive testing of all crates, transports, bridges, and edge cases.

---

## Test Coverage Tracking

### Legend
- ✅ Complete (tests written and passing)
- 🔄 In Progress
- ❌ Not Started
- ⏭️ Skipped (hardware/platform dependent)

---

## Phase 1: Core Protocol (clasp-core)

### 1.1 Codec Tests ✅
- [x] Hello message encode/decode
- [x] Welcome message encode/decode
- [x] Set message encode/decode
- [x] Publish message encode/decode
- [x] Subscribe message encode/decode
- [x] Unsubscribe message encode/decode
- [x] Ack message encode/decode
- [x] Error message encode/decode
- [x] Snapshot message encode/decode
- [x] Query message encode/decode
- [x] Get message encode/decode
- [x] Bundle message encode/decode
- [x] Ping/Pong message encode/decode

### 1.2 Value Types ✅
- [x] Null value
- [x] Bool value
- [x] Int value (i64)
- [x] Float value (f64)
- [x] String value
- [x] Array value
- [x] Map value
- [x] Bytes value

### 1.3 Address Matching ✅
- [x] Exact path matching
- [x] Single wildcard (*) matching
- [x] Multi-level wildcard (**) matching
- [x] Property access (@property)
- [x] Namespace handling
- [x] Invalid address rejection

### 1.4 Frame Encoding ✅
- [x] Frame construction with magic byte
- [x] QoS levels in frames
- [x] Timestamp handling
- [x] Large payload handling

### 1.5 State Management ✅
- [x] State store operations
- [x] Revision tracking
- [x] Lock/unlock mechanism
- [x] Pattern-based queries

---

## Phase 2: Transport Layer (clasp-transport)

### 2.1 WebSocket Transport ✅
- [x] Connection establishment
- [x] Subprotocol negotiation (clasp.v2)
- [x] Binary frame handling
- [x] Connection close
- [x] Invalid URL handling
- [x] Large message handling
- [x] Rapid connect/disconnect
- [x] Concurrent connections

### 2.2 QUIC Transport ❌
- [ ] Connection establishment
- [ ] Stream creation
- [ ] Bidirectional communication
- [ ] Connection migration
- [ ] 0-RTT reconnection
- [ ] Keep-alive handling
- [ ] Large message handling
- [ ] Concurrent streams
- [ ] TLS certificate handling
- [ ] Connection timeout

### 2.3 UDP Transport ❌
- [ ] Datagram send/receive
- [ ] Multicast support
- [ ] Broadcast support
- [ ] MTU handling
- [ ] Packet loss scenarios

### 2.4 Serial Transport ⏭️
- [ ] Connection with baud rate
- [ ] Parity settings
- [ ] Flow control
- [ ] Read/write operations
- [ ] Timeout handling
(Requires hardware - mark as integration test)

### 2.5 BLE Transport ⏭️
- [ ] GATT service discovery
- [ ] Characteristic read/write
- [ ] Notifications
- [ ] MTU negotiation
(Requires hardware - mark as integration test)

### 2.6 WebRTC Transport ❌
- [ ] Peer connection setup
- [ ] ICE candidate handling
- [ ] Data channel creation
- [ ] Message exchange
- [ ] Connection state handling

---

## Phase 3: Router (clasp-router)

### 3.1 Router Core ✅
- [x] Router creation with config
- [x] State management
- [x] WebSocket server binding

### 3.2 Session Management ❌
- [ ] Session creation on connect
- [ ] Session cleanup on disconnect
- [ ] Session timeout handling
- [ ] Max sessions limit
- [ ] Session enumeration

### 3.3 Message Routing ✅
- [x] SET message handling with ACK
- [x] Subscription delivery
- [x] Wildcard subscription matching
- [x] Multiple subscribers
- [x] State persistence across clients

### 3.4 Subscription Management ✅
- [x] Exact match subscription
- [x] Single wildcard subscription
- [x] Multi-level wildcard subscription
- [x] Unsubscribe
- [x] Multiple subscriptions per client
- [x] Initial snapshot on subscribe

### 3.5 Error Handling ✅
- [x] Malformed message handling
- [x] Truncated message handling
- [x] Wrong protocol version
- [x] Message before HELLO
- [x] Duplicate HELLO
- [x] Very long address
- [x] Empty address
- [x] Rapid disconnect/reconnect
- [x] Special characters in address

### 3.6 Advanced Router Features ❌
- [ ] Rate limiting
- [ ] Authentication/JWT validation
- [ ] Multi-transport serving
- [ ] Load balancing readiness

---

## Phase 4: Client Library (clasp-client)

### 4.1 Client Builder ❌
- [ ] Default builder
- [ ] Custom name
- [ ] Feature configuration
- [ ] Reconnection settings
- [ ] Token authentication

### 4.2 Connection Lifecycle ❌
- [ ] Connect to server
- [ ] Handshake completion
- [ ] Graceful disconnect
- [ ] Connection error handling
- [ ] Reconnection on failure

### 4.3 Parameter Operations ❌
- [ ] Set parameter
- [ ] Get parameter
- [ ] Subscribe to parameter
- [ ] Unsubscribe from parameter
- [ ] Parameter caching

### 4.4 Event Operations ❌
- [ ] Publish event
- [ ] Subscribe to events
- [ ] Event callback invocation

### 4.5 Advanced Features ❌
- [ ] Clock synchronization
- [ ] Pending request management
- [ ] Concurrent operations

---

## Phase 5: Bridge Protocols (clasp-bridge)

### 5.1 OSC Bridge ✅
- [x] Float value conversion
- [x] Integer value conversion
- [x] String value conversion
- [x] Blob value conversion
- [x] Multiple arguments
- [x] Send to external receiver
- [x] Bundle handling
- [x] High-rate message handling

### 5.2 MIDI Bridge ✅
- [x] Control Change parsing
- [x] Note On parsing
- [x] Note Off parsing
- [x] Program Change parsing
- [x] Pitch Bend parsing
- [x] SysEx parsing
- [x] Channel Pressure parsing
- [x] Poly Pressure parsing
- [x] MIDI generation
- [x] Virtual port support

### 5.3 Art-Net Bridge ✅
- [x] ArtDmx packet parsing
- [x] ArtDmx packet generation
- [x] ArtPoll parsing/generation
- [x] ArtPollReply parsing/generation
- [x] Multiple universes
- [x] DMX value range (0-255)
- [x] Sequence number handling
- [x] UDP roundtrip

### 5.4 DMX Bridge ❌
- [ ] Universe addressing
- [ ] Channel mapping
- [ ] Value scaling
- [ ] Frame rate handling

### 5.5 MQTT Bridge ❌
- [ ] Topic to address mapping
- [ ] Address to topic mapping
- [ ] QoS level handling
- [ ] Retained messages
- [ ] Connection/reconnection
- [ ] Subscription patterns

### 5.6 HTTP Bridge ❌
- [ ] GET endpoint
- [ ] POST endpoint
- [ ] PUT endpoint
- [ ] DELETE endpoint
- [ ] JSON serialization
- [ ] Error responses
- [ ] Authentication (Basic, Bearer)

### 5.7 WebSocket Bridge ❌
- [ ] Client connection
- [ ] Server mode
- [ ] Bidirectional messaging
- [ ] Connection management

### 5.8 Socket.IO Bridge ❌
- [ ] Event emission
- [ ] Event reception
- [ ] Room support
- [ ] Namespace support

### 5.9 Transform Module ❌
- [ ] Linear scaling
- [ ] Curve transforms (cubic, bezier)
- [ ] Value clamping
- [ ] Conditional transforms
- [ ] Aggregation functions

---

## Phase 6: Discovery (clasp-discovery)

### 6.1 mDNS Discovery ❌
- [ ] Service discovery
- [ ] Service advertisement
- [ ] Service registration
- [ ] Service removal
- [ ] Feature parsing

### 6.2 Broadcast Discovery ❌
- [ ] UDP broadcast send
- [ ] UDP broadcast receive
- [ ] Announcement parsing
- [ ] Device enumeration

---

## Phase 7: Embedded (clasp-embedded)

### 7.1 Lite Protocol ❌
- [ ] Lite Hello encode/decode
- [ ] Lite Welcome encode/decode
- [ ] Lite Set encode/decode
- [ ] Lite Publish encode/decode
- [ ] Lite Ping/Pong encode/decode
- [ ] 2-byte address handling
- [ ] Fixed-size messages
- [ ] no_std compatibility

---

## Phase 8: WASM Bindings (clasp-wasm)

### 8.1 WASM Client ❌
- [ ] Connection to server
- [ ] Message encoding/decoding
- [ ] Callback handlers (JS interop)
- [ ] Value conversion (Rust ↔ JS)
- [ ] Subscription management
- [ ] Error handling

---

## Phase 9: CLI Tools (clasp-cli)

### 9.1 CLI Parsing ❌
- [ ] Argument parsing
- [ ] Config file loading
- [ ] Server subcommands
- [ ] Log level configuration

---

## Phase 10: Integration & E2E

### 10.1 Multi-Protocol E2E ❌
- [ ] OSC → CLASP → MIDI
- [ ] MIDI → CLASP → Art-Net
- [ ] HTTP → CLASP → WebSocket
- [ ] Full bridge chain tests

### 10.2 Performance ✅
- [x] Encoding throughput (10K msgs)
- [x] Decoding throughput (10K msgs)
- [x] Roundtrip throughput
- [x] Large payload handling
- [x] Many small messages (50K)
- [x] Concurrent encoding
- [x] Memory stability
- [x] Latency distribution

### 10.3 Security ✅
- [x] JWT token generation
- [x] JWT token validation
- [x] Read scope enforcement
- [x] Write scope enforcement
- [x] Address constraints
- [x] Rate limit constraints
- [x] Expired token rejection
- [x] Invalid signature rejection
- [x] Wildcard scope patterns
- [x] Scope intersection

---

## Test File Locations

| Test Binary | Path | Status |
|-------------|------|--------|
| run-all-tests | test-suite/src/main.rs | ✅ |
| transport-tests | test-suite/src/bin/transport_tests.rs | ✅ |
| relay-e2e | test-suite/src/bin/relay_e2e.rs | ✅ |
| subscription-tests | test-suite/src/bin/subscription_tests.rs | ✅ |
| error-handling-tests | test-suite/src/bin/error_handling_tests.rs | ✅ |
| client-tests | test-suite/src/bin/client_tests.rs | ❌ TODO |
| discovery-tests | test-suite/src/bin/discovery_tests.rs | ❌ TODO |
| quic-tests | test-suite/src/bin/quic_tests.rs | ❌ TODO |
| bridge-tests | test-suite/src/bin/bridge_tests.rs | ❌ TODO |
| embedded-tests | test-suite/src/bin/embedded_tests.rs | ❌ TODO |

---

## Progress Summary

| Phase | Total Tests | Complete | Remaining |
|-------|-------------|----------|-----------|
| 1. Core Protocol | 40+ | 40+ | 0 |
| 2. Transport | 35 | 8 | 27 |
| 3. Router | 25 | 20 | 5 |
| 4. Client | 20 | 0 | 20 |
| 5. Bridges | 45 | 28 | 17 |
| 6. Discovery | 10 | 0 | 10 |
| 7. Embedded | 8 | 0 | 8 |
| 8. WASM | 8 | 0 | 8 |
| 9. CLI | 5 | 0 | 5 |
| 10. Integration | 20 | 18 | 2 |
| **TOTAL** | **~216** | **~114** | **~102** |

---

## Execution Order

1. ✅ Phase 1 - Core (DONE)
2. ✅ Phase 3.3-3.5 - Router messaging (DONE)
3. ✅ Phase 2.1 - WebSocket (DONE)
4. 🔄 Phase 4 - Client library (NEXT)
5. Phase 5.5-5.9 - Remaining bridges
6. Phase 6 - Discovery
7. Phase 2.2 - QUIC transport
8. Phase 7 - Embedded
9. Phase 8 - WASM
10. Phase 9 - CLI
11. Phase 10 - Final integration

---

## Post-Test Actions

- [ ] All tests passing
- [ ] Update version to 0.2.0
- [ ] Update CHANGELOG.md
- [ ] Publish to crates.io
- [ ] Publish to npm
- [ ] Publish to PyPI
- [ ] Update documentation

---

## Notes

- Serial and BLE tests require hardware - can mock or skip in CI
- WASM tests require wasm-bindgen-test setup
- Some discovery tests may need network permissions
- Target: 200+ tests total for comprehensive coverage
