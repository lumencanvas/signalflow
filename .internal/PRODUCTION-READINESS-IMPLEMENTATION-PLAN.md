# Production Readiness Implementation Plan

**Created:** 2026-01-23  
**Status:** ACTIVE  
**Goal:** Implement every claimed feature, test everything, ensure production readiness

---

## Executive Summary

This document tracks the comprehensive investigation, implementation, and testing of all CLASP features to ensure production readiness. Every claimed feature must be fully implemented, tested, and verified.

**Critical Principle:** We must not claim features we haven't implemented. Every feature in documentation must have:
1. ✅ Full implementation
2. ✅ Comprehensive tests
3. ✅ Protocol compliance verification
4. ✅ Performance validation

---

## Investigation Status

### Phase 1: Complete Feature Audit ✅ COMPLETE

**Investigation Date:** 2026-01-23  
**Status:** Complete audit performed by reading actual codebase files

**Findings (Based on Actual Code Review):**
- ✅ QUIC transport: FULLY IMPLEMENTED AND TESTED (comprehensive test suite exists)
- ✅ UDP transport: FULLY IMPLEMENTED AND TESTED (comprehensive test suite exists)
- ✅ MQTT/HTTP/WebSocket bridges: IMPLEMENTED AND TESTED (test suite exists)
- ⚠️ Gesture signal type: Codec supports it, router routes it generically (no special handling)
- ❌ Timeline signal type: Only enum exists, no message structure, no implementation
- ❌ TCP transport: Does not exist as separate transport (WebSocket uses TCP, but no standalone TCP transport)
- ✅ Late-joiner: IMPLEMENTED AND TESTED (debug_late_joiner.rs exists)
- ✅ Clock sync: IMPLEMENTED AND TESTED (clock_sync_benchmark.rs exists)
- ✅ Discovery: IMPLEMENTED AND TESTED (discovery_tests.rs exists)
- ✅ Real benchmarks: FRAMEWORK EXISTS (real_benchmarks.rs exists)

**See:** 
- `.internal/ACTUAL-IMPLEMENTATION-STATUS.md` - **DEFINITIVE STATUS** based on codebase review
- `.internal/PRODUCTION-READINESS-AUDIT.md` - Original audit (some findings were incorrect)

---

## Implementation Tracking

### Critical Features (Must Implement or Remove from Docs)

#### 1. Gesture Signal Type

**Status:** ⚠️ PARTIALLY IMPLEMENTED  
**Priority:** CRITICAL  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Types defined (`SignalType::Gesture`, `GesturePhase` enum)
- ✅ Codec can encode/decode gesture messages
- ✅ PUBLISH message supports gesture with phase and ID
- ❌ Router has no special gesture handling
- ❌ No gesture phase tracking/coalescing
- ❌ No gesture ID management
- ❌ No tests for gesture routing

**What's Missing:**
1. Router gesture handling:
   - Gesture ID tracking per address
   - Phase coalescing (keep only most recent `move`, never coalesce `start`/`end`)
   - Gesture lifecycle management
2. Tests:
   - Gesture PUBLISH encode/decode
   - Gesture routing with phase tracking
   - Gesture coalescing behavior
   - Multiple concurrent gestures
   - Gesture subscription delivery

**Implementation Tasks:**
- [ ] **INV-001:** Verify gesture codec fully works (encode/decode all phases)
- [ ] **IMPL-001:** Add gesture ID tracking to router state
- [ ] **IMPL-002:** Implement gesture phase coalescing in router
- [ ] **IMPL-003:** Add gesture lifecycle management (start → move* → end/cancel)
- [ ] **TEST-001:** Write gesture codec tests
- [ ] **TEST-002:** Write gesture routing tests
- [ ] **TEST-003:** Write gesture coalescing tests
- [ ] **TEST-004:** Write gesture subscription tests
- [ ] **VERIFY-001:** Verify gesture works end-to-end (client → router → client)

**Files to Modify:**
- `crates/clasp-router/src/router.rs` - Add gesture handling
- `crates/clasp-router/src/state.rs` - Add gesture tracking
- `crates/clasp-core/src/types.rs` - Verify types complete
- `crates/clasp-core/src/codec.rs` - Verify codec complete
- `test-suite/src/bin/gesture_tests.rs` - NEW: Comprehensive gesture tests

**Protocol Compliance:**
- Must follow CLASP-Protocol.md §4.5 Gesture specification
- Phase codes: start=0, move=1, end=2, cancel=3
- Gesture ID is stable u32 for gesture lifecycle
- Coalescing: `move` phases can be coalesced, `start`/`end`/`cancel` cannot

---

#### 2. Timeline Signal Type

**Status:** ⚠️ PARTIALLY IMPLEMENTED  
**Priority:** CRITICAL  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Types defined (`SignalType::Timeline`)
- ✅ Codec structure exists
- ❌ No timeline message structure in codec
- ❌ No time-indexed storage
- ❌ No timeline execution/scheduling
- ❌ No tests

**What's Missing:**
1. Timeline message structure:
   - Keyframes with time, value, easing
   - Loop flag
   - Start time
2. Timeline storage:
   - Time-indexed keyframe storage
   - Timeline queries by time range
3. Timeline execution:
   - Scheduled timeline playback
   - Interpolation between keyframes
   - Loop handling
4. Tests:
   - Timeline encode/decode
   - Timeline storage/retrieval
   - Timeline execution
   - Timeline subscription

**Implementation Tasks:**
- [ ] **INV-002:** Verify timeline codec structure exists
- [ ] **IMPL-004:** Design timeline message structure (keyframes, loop, startTime)
- [ ] **IMPL-005:** Implement timeline codec encode/decode
- [ ] **IMPL-006:** Add timeline storage to router state
- [ ] **IMPL-007:** Implement timeline execution engine
- [ ] **IMPL-008:** Add timeline interpolation (linear, ease-out, etc.)
- [ ] **TEST-005:** Write timeline codec tests
- [ ] **TEST-006:** Write timeline storage tests
- [ ] **TEST-007:** Write timeline execution tests
- [ ] **TEST-008:** Write timeline subscription tests
- [ ] **VERIFY-002:** Verify timeline works end-to-end

**Files to Modify:**
- `crates/clasp-core/src/types.rs` - Add Timeline message types
- `crates/clasp-core/src/codec.rs` - Add timeline encode/decode
- `crates/clasp-router/src/router.rs` - Add timeline handling
- `crates/clasp-router/src/state.rs` - Add timeline storage
- `crates/clasp-router/src/timeline.rs` - NEW: Timeline execution engine
- `test-suite/src/bin/timeline_tests.rs` - NEW: Comprehensive timeline tests

**Protocol Compliance:**
- Must follow CLASP-Protocol.md §4.6 Timeline specification
- Timestamps in microseconds
- Keyframes: { time, value, easing }
- Easing: "linear", "ease-out", "ease-in", "ease-in-out"
- Immutable once published (new timeline replaces old)

---

#### 3. TCP Transport

**Status:** ❌ NOT NEEDED (WebSocket uses TCP)  
**Priority:** LOW  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ WebSocket transport uses TCP under the hood (normal)
- ❌ No standalone TCP transport (not needed)
- ⚠️ May be mentioned in documentation incorrectly

**What's Missing:**
- Nothing - TCP is the underlying protocol for WebSocket, not a separate transport

**Implementation Tasks:**
- [ ] **INV-003:** ✅ VERIFIED - TCP is not a separate transport, WebSocket uses it
- [ ] **DOC-001:** Remove TCP from transport list in documentation (if mentioned)
- [ ] **DOC-002:** Clarify that WebSocket uses TCP as underlying protocol

**Files to Modify:**
- Documentation files only (if TCP is incorrectly listed as separate transport)

**Protocol Compliance:**
- N/A - Not a separate transport

**Decision:** Remove from implementation plan - not needed

---

#### 4. Rendezvous Server

**Status:** ❌ NOT IMPLEMENTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ❌ No rendezvous server implementation
- ✅ Documented in CLASP-Protocol.md §3.1.3
- ✅ API design exists in protocol spec

**What's Missing:**
1. Rendezvous server:
   - HTTP API server
   - Device registration endpoint
   - Device discovery endpoint
   - Public key management
   - Tag-based filtering
2. Tests:
   - Registration flow
   - Discovery flow
   - Tag filtering
   - Public key validation

**Implementation Tasks:**
- [ ] **INV-004:** Verify rendezvous server is truly missing
- [ ] **IMPL-013:** Design rendezvous server architecture
- [ ] **IMPL-014:** Implement registration endpoint (POST /api/v1/register)
- [ ] **IMPL-015:** Implement discovery endpoint (GET /api/v1/discover)
- [ ] **IMPL-016:** Add public key storage/validation
- [ ] **IMPL-017:** Add tag-based filtering
- [ ] **TEST-011:** Write rendezvous server tests
- [ ] **TEST-012:** Write rendezvous integration tests
- [ ] **VERIFY-004:** Verify rendezvous works end-to-end

**Files to Create:**
- `crates/clasp-rendezvous/` - NEW: Rendezvous server crate
- `crates/clasp-rendezvous/src/server.rs` - HTTP API server
- `crates/clasp-rendezvous/src/storage.rs` - Device storage
- `test-suite/src/bin/rendezvous_tests.rs` - NEW: Rendezvous tests

**Protocol Compliance:**
- Must follow CLASP-Protocol.md §3.1.3 Rendezvous Server specification
- POST /api/v1/register with device info
- GET /api/v1/discover?tag=... for discovery
- Public key in base64 format

**Alternative:** If rendezvous is not needed, remove from documentation

---

### Transport Testing (All Implemented, Need Tests)

#### 5. QUIC Transport Testing

**Status:** ✅ FULLY IMPLEMENTED AND TESTED  
**Priority:** ✅ COMPLETE  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ QUIC transport implemented
- ✅ TLS 1.3 support
- ✅ Connection migration
- ✅ Comprehensive test suite exists (622 lines)

**Test Coverage (Verified):**
- ✅ Configuration (default, custom)
- ✅ Client creation
- ✅ Server creation
- ✅ Connection establishment
- ✅ Bidirectional streams
- ✅ ALPN protocol

**Files:**
- `crates/clasp-transport/src/quic.rs` - Implementation exists
- `test-suite/src/bin/quic_tests.rs` - ✅ EXISTS and comprehensive

**Status:** ✅ NO ACTION NEEDED - Already fully tested

---

#### 6. UDP Transport Testing

**Status:** ✅ FULLY IMPLEMENTED AND TESTED  
**Priority:** ✅ COMPLETE  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ UDP transport implemented
- ✅ Comprehensive test suite exists (503 lines)

**Test Coverage (Verified):**
- ✅ Binding (default, config, specific port)
- ✅ Send/receive
- ✅ Broadcast
- ✅ Large packets
- ✅ Concurrent sockets
- ✅ Bidirectional communication

**Files:**
- `crates/clasp-transport/src/udp.rs` - Implementation exists
- `test-suite/src/bin/udp_tests.rs` - ✅ EXISTS and comprehensive

**Status:** ✅ NO ACTION NEEDED - Already fully tested

---

#### 7. WebRTC Transport Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ WebRTC transport implemented (webrtc-rs)
- ❌ No tests

**Testing Tasks:**
- [ ] **INV-007:** Verify WebRTC implementation is complete
- [ ] **TEST-024:** Write WebRTC peer connection setup tests
- [ ] **TEST-025:** Write WebRTC ICE candidate handling tests
- [ ] **TEST-026:** Write WebRTC data channel creation tests
- [ ] **TEST-027:** Write WebRTC message exchange tests
- [ ] **TEST-028:** Write WebRTC connection state handling tests
- [ ] **VERIFY-007:** Verify WebRTC works with router

**Files:**
- `crates/clasp-transport/src/webrtc.rs` - Implementation exists
- `test-suite/src/bin/webrtc_tests.rs` - NEW: WebRTC tests

---

#### 8. Serial Transport Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** LOW (Hardware Required)  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Serial transport implemented (tokio-serial)
- ❌ No tests
- ⚠️ Requires hardware

**Testing Tasks:**
- [ ] **INV-008:** Verify Serial implementation is complete
- [ ] **TEST-029:** Write Serial mock tests (virtual serial ports)
- [ ] **TEST-030:** Write Serial connection tests (if hardware available)
- [ ] **TEST-031:** Write Serial baud rate tests
- [ ] **TEST-032:** Write Serial timeout handling tests
- [ ] **VERIFY-008:** Verify Serial works with router (if hardware available)

**Files:**
- `crates/clasp-transport/src/serial.rs` - Implementation exists
- `test-suite/src/bin/serial_tests.rs` - NEW: Serial tests

**Note:** Can use virtual serial ports or mock for CI

---

#### 9. BLE Transport Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** LOW (Hardware Required)  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ BLE transport implemented (btleplug)
- ❌ No tests
- ⚠️ Requires hardware

**Testing Tasks:**
- [ ] **INV-009:** Verify BLE implementation is complete
- [ ] **TEST-033:** Write BLE mock tests (virtual BLE devices)
- [ ] **TEST-034:** Write BLE GATT service discovery tests
- [ ] **TEST-035:** Write BLE characteristic read/write tests
- [ ] **TEST-036:** Write BLE notifications tests
- [ ] **TEST-037:** Write BLE MTU negotiation tests
- [ ] **VERIFY-009:** Verify BLE works with router (if hardware available)

**Files:**
- `crates/clasp-transport/src/ble.rs` - Implementation exists
- `test-suite/src/bin/ble_tests.rs` - NEW: BLE tests

**Note:** Can use virtual BLE devices or mock for CI

---

### Bridge Testing (All Implemented, Need Tests)

#### 10. MQTT Bridge Testing

**Status:** ✅ IMPLEMENTED, ⚠️ BASIC TESTS EXIST  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ MQTT bridge implemented
- ✅ v3.1.1/v5 support
- ✅ TLS support
- ✅ Basic tests exist (config, creation)

**Testing Tasks:**
- [ ] **INV-010:** ✅ VERIFIED - Basic tests exist
- [ ] **TEST-038:** Write MQTT topic to address mapping tests
- [ ] **TEST-039:** Write MQTT address to topic mapping tests
- [ ] **TEST-040:** Write MQTT QoS level handling tests
- [ ] **TEST-041:** Write MQTT retained messages tests
- [ ] **TEST-042:** Write MQTT connection/reconnection tests
- [ ] **TEST-043:** Write MQTT subscription pattern tests
- [ ] **TEST-044:** Write MQTT TLS tests
- [ ] **VERIFY-010:** Verify MQTT bridge works end-to-end

**Files:**
- `crates/clasp-bridge/src/mqtt.rs` - Implementation exists
- `test-suite/src/bin/bridge_tests.rs` - ✅ EXISTS with basic MQTT tests (needs expansion)

---

#### 11. HTTP Bridge Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ HTTP bridge implemented
- ✅ REST API
- ✅ CORS support
- ❌ No integration tests

**Testing Tasks:**
- [ ] **INV-011:** Verify HTTP bridge implementation is complete
- [ ] **TEST-045:** Write HTTP GET endpoint tests
- [ ] **TEST-046:** Write HTTP POST endpoint tests
- [ ] **TEST-047:** Write HTTP PUT endpoint tests
- [ ] **TEST-048:** Write HTTP DELETE endpoint tests
- [ ] **TEST-049:** Write HTTP JSON serialization tests
- [ ] **TEST-050:** Write HTTP error response tests
- [ ] **TEST-051:** Write HTTP authentication tests (Basic, Bearer)
- [ ] **TEST-052:** Write HTTP CORS tests
- [ ] **VERIFY-011:** Verify HTTP bridge works end-to-end

**Files:**
- `crates/clasp-bridge/src/http.rs` - Implementation exists
- `test-suite/src/bin/bridge_tests.rs` - EXISTS but may not have HTTP tests

---

#### 12. WebSocket Bridge Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ WebSocket bridge implemented
- ✅ Client/server modes
- ✅ JSON/MsgPack formats
- ❌ No integration tests

**Testing Tasks:**
- [ ] **INV-012:** Verify WebSocket bridge implementation is complete
- [ ] **TEST-053:** Write WebSocket client connection tests
- [ ] **TEST-054:** Write WebSocket server mode tests
- [ ] **TEST-055:** Write WebSocket bidirectional messaging tests
- [ ] **TEST-056:** Write WebSocket connection management tests
- [ ] **TEST-057:** Write WebSocket JSON format tests
- [ ] **TEST-058:** Write WebSocket MsgPack format tests
- [ ] **VERIFY-012:** Verify WebSocket bridge works end-to-end

**Files:**
- `crates/clasp-bridge/src/websocket.rs` - Implementation exists
- `test-suite/src/bin/bridge_tests.rs` - EXISTS but may not have WebSocket bridge tests

---

#### 13. Socket.IO Bridge Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Socket.IO bridge implemented
- ✅ v4 support
- ❌ No integration tests

**Testing Tasks:**
- [ ] **INV-013:** Verify Socket.IO bridge implementation is complete
- [ ] **TEST-059:** Write Socket.IO event emission tests
- [ ] **TEST-060:** Write Socket.IO event reception tests
- [ ] **TEST-061:** Write Socket.IO room support tests
- [ ] **TEST-062:** Write Socket.IO namespace support tests
- [ ] **VERIFY-013:** Verify Socket.IO bridge works end-to-end

**Files:**
- `crates/clasp-bridge/src/socketio.rs` - Implementation exists
- `test-suite/src/bin/bridge_tests.rs` - EXISTS but may not have Socket.IO tests

---

#### 14. sACN Bridge Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ sACN bridge implemented
- ✅ Multiple modes
- ❌ No integration tests

**Testing Tasks:**
- [ ] **INV-014:** Verify sACN bridge implementation is complete
- [ ] **TEST-063:** Write sACN universe addressing tests
- [ ] **TEST-064:** Write sACN channel mapping tests
- [ ] **TEST-065:** Write sACN priority handling tests
- [ ] **TEST-066:** Write sACN multicast tests
- [ ] **VERIFY-014:** Verify sACN bridge works end-to-end

**Files:**
- `crates/clasp-bridge/src/sacn.rs` - Implementation exists
- `test-suite/src/bin/bridge_tests.rs` - EXISTS but may not have sACN tests

---

#### 15. DMX Bridge Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** LOW (Hardware Required)  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ DMX bridge implemented
- ✅ Output only
- ✅ USB interfaces
- ❌ No integration tests
- ⚠️ Requires hardware (ENTTEC Pro, FTDI)

**Testing Tasks:**
- [ ] **INV-015:** Verify DMX bridge implementation is complete
- [ ] **TEST-067:** Write DMX universe addressing tests
- [ ] **TEST-068:** Write DMX channel mapping tests
- [ ] **TEST-069:** Write DMX value scaling tests
- [ ] **TEST-070:** Write DMX frame rate handling tests
- [ ] **TEST-071:** Write DMX hardware interface tests (if hardware available)
- [ ] **VERIFY-015:** Verify DMX bridge works end-to-end (if hardware available)

**Files:**
- `crates/clasp-bridge/src/dmx.rs` - Implementation exists (has TODOs for ENTTEC Pro, FTDI)
- `test-suite/src/bin/bridge_tests.rs` - EXISTS but may not have DMX tests

**Note:** Can use virtual DMX interfaces or mock for CI

---

### Advanced Features Testing

#### 16. Late-Joiner Support Testing

**Status:** ✅ FULLY IMPLEMENTED AND TESTED  
**Priority:** ✅ COMPLETE  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Router sends snapshot on connection
- ✅ Snapshot includes all current param values
- ✅ Chunked if too large
- ✅ Comprehensive test exists

**Test Coverage (Verified):**
- ✅ Snapshot on connect
- ✅ Chunking for large state (500, 1000, 2000, 5000, 10000 params)
- ✅ Performance measurement

**Files:**
- `crates/clasp-router/src/router.rs` - Implementation exists
- `test-suite/src/bin/debug_late_joiner.rs` - ✅ EXISTS and comprehensive

**Status:** ✅ NO ACTION NEEDED - Already fully tested

---

#### 17. Clock Synchronization Testing

**Status:** ✅ FULLY IMPLEMENTED AND TESTED  
**Priority:** ✅ COMPLETE  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ SYNC message handling exists
- ✅ NTP-like algorithm implemented
- ✅ Timestamp tracking
- ✅ Comprehensive benchmark exists

**Test Coverage (Verified):**
- ✅ Clock sync accuracy (LAN, WiFi, WAN scenarios)
- ✅ Jitter measurement
- ✅ Convergence speed
- ✅ Real-time jitter measurement

**Files:**
- `crates/clasp-core/src/time.rs` - Implementation exists (ClockSync struct)
- `test-suite/src/bin/clock_sync_benchmark.rs` - ✅ EXISTS and comprehensive

**Status:** ✅ NO ACTION NEEDED - Already fully tested

---

#### 18. Bundle (Atomic) Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Bundle message structure exists
- ✅ Scheduled bundle support
- ❌ No tests verify atomicity
- ❌ No tests verify scheduled execution
- ❌ No tests verify bundle ordering

**Testing Tasks:**
- [ ] **INV-018:** Verify bundle implementation is complete
- [ ] **TEST-082:** Write bundle atomicity tests
- [ ] **TEST-083:** Write bundle scheduled execution tests
- [ ] **TEST-084:** Write bundle ordering tests
- [ ] **TEST-085:** Write bundle with multiple messages tests
- [ ] **TEST-086:** Write bundle timestamp handling tests
- [ ] **VERIFY-018:** Verify bundles work correctly

**Files:**
- `crates/clasp-core/src/types.rs` - Bundle message type exists
- `crates/clasp-router/src/router.rs` - Bundle handling exists
- `test-suite/src/bin/protocol_tests.rs` - EXISTS but may not have bundle tests

---

#### 19. QoS Levels Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ QoS levels defined (Fire, Confirm, Commit)
- ✅ Default QoS per signal type
- ❌ No tests verify QoS behavior
- ❌ No tests verify retransmission
- ❌ No tests verify ordering

**Testing Tasks:**
- [ ] **INV-019:** Verify QoS implementation is complete
- [ ] **TEST-087:** Write QoS Fire (best effort) tests
- [ ] **TEST-088:** Write QoS Confirm (at least once) tests
- [ ] **TEST-089:** Write QoS Commit (exactly once, ordered) tests
- [ ] **TEST-090:** Write QoS retransmission tests
- [ ] **TEST-091:** Write QoS ordering tests
- [ ] **VERIFY-019:** Verify QoS works correctly

**Files:**
- `crates/clasp-core/src/types.rs` - QoS enum exists
- `crates/clasp-router/src/router.rs` - QoS handling exists
- `test-suite/src/bin/protocol_tests.rs` - EXISTS but may not have QoS tests

---

#### 20. Stream Signal Type Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Stream message structure exists
- ✅ Routing exists
- ⚠️ Coalescing logic unclear
- ❌ No tests

**Testing Tasks:**
- [ ] **INV-020:** Verify stream implementation is complete
- [ ] **TEST-092:** Write stream PUBLISH encode/decode tests
- [ ] **TEST-093:** Write stream routing tests
- [ ] **TEST-094:** Write stream coalescing tests
- [ ] **TEST-095:** Write stream subscription tests
- [ ] **TEST-096:** Write stream high-rate tests
- [ ] **VERIFY-020:** Verify streams work correctly

**Files:**
- `crates/clasp-core/src/types.rs` - Stream type exists
- `crates/clasp-router/src/router.rs` - Stream handling exists
- `test-suite/src/bin/protocol_tests.rs` - EXISTS but may not have stream tests

---

### Performance & Stress Testing

#### 21. Real Benchmarks (from HARDENING-PLAN.md)

**Status:** ✅ FRAMEWORK EXISTS, ⚠️ NEEDS VALIDATION  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Comprehensive benchmark framework exists (`test-suite/src/bin/real_benchmarks.rs` - 611 lines)
- ✅ All scenarios from HARDENING-PLAN.md implemented
- ⚠️ Benchmarks need validation runs
- ⚠️ No baseline numbers documented

**Testing Tasks:**
- [ ] **INV-021:** Verify benchmark framework is complete
- [ ] **TEST-097:** Run Scenario A: End-to-End Single Hop
- [ ] **TEST-098:** Run Scenario B: Fanout Curve (1, 10, 50, 100, 500, 1000)
- [ ] **TEST-099:** Run Scenario C: Address Table Scale (100, 1k, 10k, 100k)
- [ ] **TEST-100:** Run Scenario D: Wildcard Routing Cost
- [ ] **TEST-101:** Run Scenario E: Feature Toggle Matrix
- [ ] **TEST-102:** Run Scenario F: Bridge Overhead
- [ ] **VERIFY-021:** Document baseline numbers

**Files:**
- `test-suite/src/bin/real_benchmarks.rs` - EXISTS
- `HARDENING-PLAN.md` - Has benchmark scenarios

---

#### 22. Stress Tests (from HARDENING-PLAN.md)

**Status:** ⚠️ FRAMEWORK EXISTS, ❌ NOT VALIDATED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Stress test framework exists
- ❌ Tests not validated
- ❌ No results documented

**Testing Tasks:**
- [ ] **INV-022:** Verify stress test framework is complete
- [ ] **TEST-103:** Run 10k address scale test
- [ ] **TEST-104:** Run 1000 subscriber fanout test
- [ ] **TEST-105:** Run late-joiner replay storm test
- [ ] **TEST-106:** Run scheduled bundle cascade test
- [ ] **TEST-107:** Run backpressure behavior test
- [ ] **TEST-108:** Run clock sync accuracy test
- [ ] **VERIFY-022:** Document stress test results

**Files:**
- `test-suite/src/bin/stress_tests.rs` - May need to create
- `HARDENING-PLAN.md` - Has stress test scenarios

---

### Security Testing

#### 23. Rate Limiting Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ maxRate in constraints
- ❌ No enforcement tests

**Testing Tasks:**
- [ ] **INV-023:** Verify rate limiting implementation is complete
- [ ] **TEST-109:** Write rate limiting enforcement tests
- [ ] **TEST-110:** Write rate limiting per-address tests
- [ ] **TEST-111:** Write rate limiting per-session tests
- [ ] **TEST-112:** Write rate limiting error handling tests
- [ ] **VERIFY-023:** Verify rate limiting works correctly

**Files:**
- `crates/clasp-router/src/router.rs` - Rate limiting exists
- `test-suite/src/bin/security_tests.rs` - EXISTS but may not have rate limiting tests

---

#### 24. Capability Scopes Testing

**Status:** ✅ IMPLEMENTED, ⚠️ PARTIALLY TESTED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ Read/write scopes
- ✅ Address patterns
- ✅ Constraints (range, maxRate)
- ⚠️ Basic tests exist
- ❌ Need comprehensive tests

**Testing Tasks:**
- [ ] **INV-024:** Verify capability scopes implementation is complete
- [ ] **TEST-113:** Write scope read enforcement tests
- [ ] **TEST-114:** Write scope write enforcement tests
- [ ] **TEST-115:** Write scope wildcard pattern tests
- [ ] **TEST-116:** Write scope constraint tests (range, maxRate)
- [ ] **TEST-117:** Write scope intersection tests
- [ ] **VERIFY-024:** Verify capability scopes work correctly

**Files:**
- `crates/clasp-router/src/router.rs` - Scope enforcement exists
- `test-suite/src/bin/security_tests.rs` - EXISTS but may need expansion

---

#### 25. TLS/Encryption Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** HIGH  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ WSS support
- ✅ QUIC TLS 1.3
- ❌ No encryption tests

**Testing Tasks:**
- [ ] **INV-025:** Verify TLS implementation is complete
- [ ] **TEST-118:** Write WSS encryption tests
- [ ] **TEST-119:** Write QUIC TLS 1.3 tests
- [ ] **TEST-120:** Write certificate validation tests
- [ ] **TEST-121:** Write TLS handshake tests
- [ ] **VERIFY-025:** Verify TLS works correctly

**Files:**
- `crates/clasp-transport/src/websocket.rs` - WSS support exists
- `crates/clasp-transport/src/quic.rs` - TLS 1.3 exists
- `test-suite/src/bin/security_tests.rs` - EXISTS but may not have TLS tests

---

### Discovery Testing

#### 26. mDNS Discovery Testing

**Status:** ✅ IMPLEMENTED, ⚠️ PARTIALLY TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ mDNS implementation (mdns-sd crate)
- ✅ Service type: `_clasp._tcp.local`
- ✅ TXT records
- ⚠️ Basic tests exist
- ❌ Need comprehensive tests

**Testing Tasks:**
- [ ] **INV-026:** Verify mDNS implementation is complete
- [ ] **TEST-122:** Write mDNS service discovery tests
- [ ] **TEST-123:** Write mDNS service advertisement tests
- [ ] **TEST-124:** Write mDNS service registration tests
- [ ] **TEST-125:** Write mDNS service removal tests
- [ ] **TEST-126:** Write mDNS feature parsing tests
- [ ] **VERIFY-026:** Verify mDNS works correctly

**Files:**
- `crates/clasp-discovery/src/mdns.rs` - Implementation exists
- `test-suite/src/bin/discovery_tests.rs` - EXISTS but may be incomplete

---

#### 27. UDP Broadcast Discovery Testing

**Status:** ✅ IMPLEMENTED, ❌ NOT TESTED  
**Priority:** MEDIUM  
**Investigation Date:** 2026-01-23

**Current State:**
- ✅ UDP broadcast implementation
- ✅ Port 7331
- ✅ HELLO/ANNOUNCE protocol
- ❌ No tests

**Testing Tasks:**
- [ ] **INV-027:** Verify UDP broadcast implementation is complete
- [ ] **TEST-127:** Write UDP broadcast send tests
- [ ] **TEST-128:** Write UDP broadcast receive tests
- [ ] **TEST-129:** Write UDP broadcast announcement parsing tests
- [ ] **TEST-130:** Write UDP broadcast device enumeration tests
- [ ] **VERIFY-027:** Verify UDP broadcast works correctly

**Files:**
- `crates/clasp-discovery/src/broadcast.rs` - Implementation exists
- `test-suite/src/bin/discovery_tests.rs` - EXISTS but may not have broadcast tests

---

## Implementation Priority

### Phase 1: Critical Features (Weeks 1-2)
1. Gesture signal type implementation
2. Timeline signal type implementation
3. TCP transport implementation (or remove from docs)
4. Late-joiner support testing
5. Clock synchronization testing
6. Bundle testing
7. QoS levels testing

### Phase 2: Transport Testing (Weeks 3-4)
1. QUIC transport testing
2. UDP transport testing
3. WebRTC transport testing
4. Serial/BLE transport testing (with mocks)

### Phase 3: Bridge Testing (Weeks 5-6)
1. MQTT bridge testing
2. HTTP bridge testing
3. WebSocket bridge testing
4. Socket.IO bridge testing
5. sACN bridge testing
6. DMX bridge testing (with mocks)

### Phase 4: Advanced Features (Weeks 7-8)
1. Stream signal type testing
2. Rate limiting testing
3. Capability scopes comprehensive testing
4. TLS/encryption testing
5. mDNS discovery comprehensive testing
6. UDP broadcast discovery testing

### Phase 5: Performance & Stress (Weeks 9-10)
1. Real benchmarks validation
2. Stress tests validation
3. Performance documentation

### Phase 6: Rendezvous (Optional, Weeks 11-12)
1. Rendezvous server implementation (or remove from docs)
2. Rendezvous server testing

---

## Task Tracking Format

Each task follows this format:
- **INV-XXX:** Investigation task (verify what exists)
- **IMPL-XXX:** Implementation task (write code)
- **TEST-XXX:** Testing task (write tests)
- **VERIFY-XXX:** Verification task (end-to-end validation)

---

## Success Criteria

### For Each Feature:
1. ✅ Implementation complete and working
2. ✅ Tests written and passing
3. ✅ Protocol compliance verified
4. ✅ Performance validated (if applicable)
5. ✅ Documentation updated

### Overall:
1. ✅ All claimed features implemented
2. ✅ 80%+ test coverage
3. ✅ All tests passing
4. ✅ Performance benchmarks documented
5. ✅ Security audit complete
6. ✅ Documentation accurate

---

## Notes

- **Hardware Requirements:** Serial, BLE, and DMX tests require hardware. Use mocks/virtual devices for CI.
- **Protocol Compliance:** All implementations must strictly follow CLASP-Protocol.md
- **Performance:** All features must meet performance targets from HARDENING-PLAN.md
- **No Shortcuts:** Every feature must be fully implemented, not stubbed

---

**Last Updated:** 2026-01-23  
**Next Review:** After Phase 1 completion
