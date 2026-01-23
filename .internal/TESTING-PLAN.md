# CLASP Testing Plan
**Date:** January 23, 2026  
**Status:** 📋 **PLANNING**

---

## Executive Summary

This document outlines the comprehensive testing plan for CLASP, identifying gaps from the audit and prioritizing test development to ensure full protocol implementation verification.

**Current Status:** 66 test files exist, covering core functionality well. Gaps identified in advanced features, some bridges, and transports.

---

## Part 1: High Priority Tests (Critical Gaps)

### 1.1 P2P WebRTC Connection Tests ⚠️ IN PROGRESS

**Status:** ICE candidate fix implemented, needs verification

**Required Tests:**

1. **Basic P2P Connection**
   - [ ] Two clients connect via router signaling
   - [ ] ICE candidates exchanged successfully
   - [ ] DataChannel established
   - [ ] Connection state transitions (connecting → connected)
   - [ ] Messages flow bidirectionally

2. **NAT Traversal Scenarios**
   - [ ] Same network (no NAT)
   - [ ] Different networks (NAT traversal required)
   - [ ] Symmetric NAT (most restrictive)
   - [ ] STUN server configuration
   - [ ] TURN server fallback

3. **Connection Resilience**
   - [ ] Connection recovery after network interruption
   - [ ] Reconnection with existing session
   - [ ] Multiple P2P connections simultaneously
   - [ ] Connection timeout handling

4. **DataChannel Reliability**
   - [ ] Reliable channel (ordered: true) for Q1/Q2 messages
   - [ ] Unreliable channel (ordered: false) for Q0 streams
   - [ ] Message ordering verification
   - [ ] Packet loss handling

5. **Performance**
   - [ ] Latency measurement (should be <10ms)
   - [ ] Throughput test (messages/second)
   - [ ] Bandwidth usage

**Test Files:**
- `test-suite/src/bin/p2p_connection_tests.rs` (expand existing)
- `test-suite/src/bin/p2p_nat_traversal_tests.rs` (new)
- `test-suite/src/bin/p2p_resilience_tests.rs` (new)

**Priority:** 🔴 **CRITICAL** - P2P is a core protocol promise

---

### 1.2 Bridge Integration Tests

**Status:** Some bridges have tests, others need comprehensive integration tests

#### 1.2.1 MQTT Bridge ⚠️ NEEDS MORE

**Required Tests:**
- [ ] MQTT → CLASP message translation
- [ ] CLASP → MQTT message translation
- [ ] QoS level mapping (MQTT QoS 0/1/2 → CLASP Q0/Q1/Q2)
- [ ] Topic → Address mapping
- [ ] Retained messages → CLASP state
- [ ] Will messages handling
- [ ] Multiple MQTT brokers
- [ ] Authentication (username/password, certificates)

**Test File:** `test-suite/src/bin/mqtt_integration_tests.rs` (expand existing)

**Priority:** 🟠 **HIGH** - MQTT is common in IoT scenarios

#### 1.2.2 HTTP Bridge ⚠️ NEEDS MORE

**Required Tests:**
- [ ] REST API → CLASP SET/PUBLISH
- [ ] CLASP → HTTP webhook callbacks
- [ ] GET requests → CLASP GET
- [ ] POST/PUT/PATCH → CLASP SET
- [ ] DELETE → CLASP SET (null value)
- [ ] Query parameters → CLASP address mapping
- [ ] Request body parsing (JSON, form-data)
- [ ] Response formatting
- [ ] Authentication (API keys, OAuth)
- [ ] Rate limiting
- [ ] CORS handling

**Test File:** `test-suite/src/bin/http_integration_tests.rs` (new)

**Priority:** 🟠 **HIGH** - HTTP is universal, needed for web integration

#### 1.2.3 WebSocket Bridge ⚠️ NEEDS MORE

**Required Tests:**
- [ ] WebSocket → CLASP message translation
- [ ] CLASP → WebSocket message translation
- [ ] Text vs binary frames
- [ ] JSON payload parsing
- [ ] MessagePack payload parsing
- [ ] Subprotocol negotiation
- [ ] Connection lifecycle
- [ ] Reconnection handling

**Test File:** `test-suite/src/bin/websocket_bridge_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - WebSocket is common but CLASP native transport also uses it

#### 1.2.4 Socket.IO Bridge ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Socket.IO events → CLASP PUBLISH
- [ ] CLASP → Socket.IO events
- [ ] Room/namespace mapping
- [ ] Acknowledgment handling
- [ ] Binary data handling

**Test File:** `test-suite/src/bin/socketio_bridge_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - Socket.IO is less common but supported

#### 1.2.5 sACN Bridge ❌ NOT TESTED

**Required Tests:**
- [ ] sACN → CLASP SET (universe/channel → address)
- [ ] CLASP → sACN (address → universe/channel)
- [ ] Priority handling
- [ ] Multicast vs unicast
- [ ] Universe discovery
- [ ] Multiple universes

**Test File:** `test-suite/src/bin/sacn_integration_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - sACN is professional lighting standard

#### 1.2.6 DMX Bridge ⚠️ NEEDS MORE

**Required Tests:**
- [ ] DMX → CLASP SET
- [ ] CLASP → DMX
- [ ] Universe handling
- [ ] Channel mapping
- [ ] Refresh rate
- [ ] Hardware interface (FTDI, Enttec, etc.)

**Test File:** `test-suite/src/bin/dmx_integration_tests.rs` (expand existing)

**Priority:** 🟡 **MEDIUM** - DMX is core lighting protocol

---

### 1.3 Transport Tests

#### 1.3.1 TCP Transport ⚠️ NEEDS MORE

**Required Tests:**
- [ ] TCP connection establishment
- [ ] Frame boundary detection
- [ ] Connection recovery
- [ ] Multiple concurrent connections
- [ ] Large message handling (65KB max)
- [ ] Keepalive/ping handling
- [ ] TLS encryption

**Test File:** `test-suite/src/bin/tcp_transport_tests.rs` (new)

**Priority:** 🟠 **HIGH** - TCP is reliable transport option

#### 1.3.2 Serial Transport ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Serial port connection
- [ ] Baud rate configuration
- [ ] Frame encoding/decoding
- [ ] Flow control
- [ ] Error detection/correction
- [ ] Hardware loopback test

**Test File:** `test-suite/src/bin/serial_transport_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - Serial is for hardware integration

#### 1.3.3 BLE Transport ❌ NOT TESTED

**Required Tests:**
- [ ] BLE device discovery
- [ ] Connection establishment
- [ ] Characteristic read/write
- [ ] Notification handling
- [ ] MTU negotiation
- [ ] Connection timeout
- [ ] Multiple devices

**Test File:** `test-suite/src/bin/ble_transport_tests.rs` (new)

**Priority:** 🟢 **LOW** - BLE is for battery-powered devices

---

## Part 2: Advanced Feature Tests

### 2.1 BUNDLE Message Tests ⚠️ NEEDS MORE

**Current:** Basic tests exist, need comprehensive coverage

**Required Tests:**
- [ ] Atomic execution (all or nothing)
- [ ] Scheduled execution (timestamp-based)
- [ ] Mixed message types in bundle
- [ ] Large bundles (many messages)
- [ ] Bundle within bundle (nested)
- [ ] Partial failure handling
- [ ] Timestamp precision
- [ ] Clock sync for scheduled bundles

**Test File:** `test-suite/src/bin/bundle_tests.rs` (expand existing)

**Priority:** 🟠 **HIGH** - BUNDLE is core protocol feature

---

### 2.2 QUERY Message Tests ⚠️ NEEDS MORE

**Current:** Basic tests exist, need comprehensive coverage

**Required Tests:**
- [ ] Query by pattern (`/lumen/**`)
- [ ] Query by type (param, event, stream, etc.)
- [ ] Query with filters
- [ ] Response formatting
- [ ] Large result sets
- [ ] Performance (query speed)
- [ ] Nested queries

**Test File:** `test-suite/src/bin/query_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - QUERY is useful for introspection

---

### 2.3 ANNOUNCE Message Tests ⚠️ NEEDS MORE

**Current:** Basic tests exist, need comprehensive coverage

**Required Tests:**
- [ ] Signal announcement on connect
- [ ] Dynamic announcement (add/remove signals)
- [ ] Namespace organization
- [ ] Metadata inclusion
- [ ] Bridge announcements
- [ ] Announcement updates
- [ ] Multiple namespaces

**Test File:** `test-suite/src/bin/announce_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - ANNOUNCE enables discovery

---

### 2.4 State Management Advanced Tests

#### 2.4.1 Conflict Resolution ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Last-write-wins (lww) strategy
- [ ] Max value strategy
- [ ] Min value strategy
- [ ] Lock strategy (exclusive control)
- [ ] Merge strategy (application-defined)
- [ ] Concurrent writes from multiple clients
- [ ] Revision number handling
- [ ] Timestamp-based resolution

**Test File:** `test-suite/src/bin/conflict_resolution_tests.rs` (new)

**Priority:** 🟠 **HIGH** - Conflict resolution is critical for multi-controller scenarios

#### 2.4.2 Lock/Unlock ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Lock acquisition
- [ ] Lock denial (already locked)
- [ ] Lock holder information
- [ ] Lock timeout
- [ ] Lock release
- [ ] Force unlock (admin)
- [ ] Multiple locks simultaneously
- [ ] Lock on disconnect

**Test File:** `test-suite/src/bin/lock_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - Locks prevent conflicts

---

### 2.5 Stream Signal Type Tests ⚠️ NEEDS MORE

**Current:** Basic tests exist, need comprehensive coverage

**Required Tests:**
- [ ] High-rate streaming (100+ Hz)
- [ ] Rate limiting (maxRate option)
- [ ] Change threshold (epsilon option)
- [ ] Batching (window option)
- [ ] Packet loss tolerance
- [ ] Stream subscription options
- [ ] Multiple concurrent streams
- [ ] Stream vs Param distinction

**Test File:** `test-suite/src/bin/stream_tests.rs` (expand existing)

**Priority:** 🟠 **HIGH** - Streams are core signal type

---

## Part 3: Edge Cases and Error Handling

### 3.1 Error Handling Tests

**Required Tests:**
- [ ] Invalid message format
- [ ] Unknown message type
- [ ] Invalid address format
- [ ] Type mismatches
- [ ] Out-of-range values
- [ ] Missing required fields
- [ ] Malformed binary encoding
- [ ] MessagePack compatibility errors
- [ ] Protocol version mismatch
- [ ] Encoding version mismatch

**Test File:** `test-suite/src/bin/error_handling_tests.rs` (expand existing)

**Priority:** 🟠 **HIGH** - Robust error handling is critical

---

### 3.2 Network Edge Cases

**Required Tests:**
- [ ] Network partition recovery
- [ ] Packet reordering
- [ ] Duplicate messages
- [ ] Out-of-order delivery
- [ ] Connection timeout
- [ ] Rapid connect/disconnect
- [ ] Message flooding (DoS protection)
- [ ] Large message handling
- [ ] Fragmentation (if applicable)

**Test File:** `test-suite/src/bin/network_edge_cases_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - Network issues are common

---

### 3.3 Race Condition Tests

**Required Tests:**
- [ ] Concurrent SET operations
- [ ] Subscribe during publish
- [ ] Unsubscribe during publish
- [ ] Connect during state update
- [ ] Disconnect during message send
- [ ] Multiple routers (split-brain)

**Test File:** `test-suite/src/bin/race_condition_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - Race conditions cause subtle bugs

---

## Part 4: Performance and Stress Tests

### 4.1 Load Tests ⚠️ EXISTS, EXPAND

**Current:** Basic load tests exist

**Required Tests:**
- [ ] 1000+ concurrent connections
- [ ] 10,000+ messages/second
- [ ] Large fanout (1000+ subscribers)
- [ ] Memory usage under load
- [ ] CPU usage under load
- [ ] Connection establishment rate
- [ ] Message queue depth
- [ ] Router throughput limits

**Test File:** `test-suite/src/bin/load_tests.rs` (expand existing)

**Priority:** 🟡 **MEDIUM** - Performance is important but not critical

---

### 4.2 Soak Tests ⚠️ EXISTS, EXPAND

**Current:** Basic soak tests exist

**Required Tests:**
- [ ] 24-hour continuous operation
- [ ] Memory leak detection
- [ ] Connection stability
- [ ] State consistency over time
- [ ] Clock drift detection
- [ ] Resource cleanup

**Test File:** `test-suite/src/bin/soak_tests.rs` (expand existing)

**Priority:** 🟢 **LOW** - Soak tests are important but time-consuming

---

## Part 5: Security Tests

### 5.1 Encryption Tests ⚠️ NEEDS MORE

**Required Tests:**
- [ ] TLS 1.3 for WebSocket
- [ ] DTLS for UDP/DataChannel
- [ ] Certificate validation
- [ ] Self-signed certificate handling
- [ ] Certificate pinning
- [ ] Cipher suite negotiation
- [ ] Perfect forward secrecy

**Test File:** `test-suite/src/bin/encryption_tests.rs` (new)

**Priority:** 🟠 **HIGH** - Encryption is security requirement

---

### 5.2 Authentication Tests ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Capability token validation
- [ ] Token expiration
- [ ] Token refresh
- [ ] Permission checking (read/write)
- [ ] Address pattern matching in tokens
- [ ] Constraint validation (range, maxRate)
- [ ] Token revocation
- [ ] Multiple tokens per session

**Test File:** `test-suite/src/bin/authentication_tests.rs` (expand existing)

**Priority:** 🟠 **HIGH** - Authentication is security requirement

---

### 5.3 Penetration Tests ⚠️ EXISTS, EXPAND

**Current:** Basic penetration tests exist

**Required Tests:**
- [ ] Injection attacks (address, value)
- [ ] Buffer overflow attempts
- [ ] Rate limit bypass attempts
- [ ] Token forgery attempts
- [ ] Replay attacks
- [ ] Man-in-the-middle attacks
- [ ] DoS attacks (message flooding)
- [ ] Resource exhaustion

**Test File:** `test-suite/src/bin/security_pentest.rs` (expand existing)

**Priority:** 🟠 **HIGH** - Security is critical

---

## Part 6: Language Binding Tests

### 6.1 JavaScript/TypeScript Tests ✅ GOOD

**Status:** Comprehensive tests exist

**Additional Tests Needed:**
- [ ] Browser vs Node.js differences
- [ ] WebRTC DataChannel in browser
- [ ] WASM performance
- [ ] Type definitions accuracy

**Priority:** 🟢 **LOW** - Already well-tested

---

### 6.2 Python Tests ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Async vs sync API
- [ ] Thread safety
- [ ] GIL impact
- [ ] Memory management
- [ ] Exception handling
- [ ] Type hints accuracy

**Test File:** `bindings/python/tests/test_client.py` (expand existing)

**Priority:** 🟡 **MEDIUM** - Python is common in creative tools

---

### 6.3 Rust Tests ✅ EXCELLENT

**Status:** Comprehensive tests exist

**Additional Tests Needed:**
- [ ] `no_std` embedded tests
- [ ] WASM compilation
- [ ] Cross-platform compatibility

**Priority:** 🟢 **LOW** - Already well-tested

---

### 6.4 WASM Tests ⚠️ NEEDS MORE

**Required Tests:**
- [ ] Browser compatibility
- [ ] WebWorker support
- [ ] Memory limits
- [ ] Performance benchmarks
- [ ] Size optimization

**Test File:** `crates/clasp-wasm/tests/web.rs` (expand existing)

**Priority:** 🟡 **MEDIUM** - WASM enables browser usage

---

## Part 7: Integration Test Scenarios

### 7.1 Real-World Scenarios

**Required Tests:**
- [ ] Live performance setup (OSC + MIDI + DMX)
- [ ] Installation art (MQTT + OSC + Art-Net)
- [ ] Home automation (HTTP + MQTT)
- [ ] Software integration (WebSocket + OSC + MIDI)
- [ ] Multi-router setup
- [ ] Router failover

**Test File:** `test-suite/src/bin/real_world_scenarios_tests.rs` (new)

**Priority:** 🟡 **MEDIUM** - Real-world validation

---

## Part 8: Test Execution Plan

### 8.1 Test Organization

**Structure:**
```
test-suite/src/bin/
├── p2p_*.rs              # P2P tests
├── *bridge_tests.rs       # Bridge tests
├── *transport_tests.rs    # Transport tests
├── *integration_tests.rs  # Integration tests
└── ...
```

### 8.2 Test Execution

**CI/CD Integration:**
- [ ] Run all tests on every commit
- [ ] Run performance tests on schedule
- [ ] Run soak tests on schedule
- [ ] Run security tests on schedule
- [ ] Generate coverage reports

**Local Execution:**
```bash
# Run all tests
cargo test --workspace

# Run specific test suite
cargo run --bin p2p_connection_tests

# Run with logging
RUST_LOG=info cargo run --bin p2p_connection_tests
```

### 8.3 Test Coverage Goals

**Target Coverage:**
- Core protocol: 95%+
- Bridges: 80%+
- Transports: 80%+
- Client libraries: 90%+
- Router: 90%+

**Tools:**
- `cargo tarpaulin` for Rust coverage
- Manual review for language bindings

---

## Part 9: Priority Summary

### 🔴 Critical (Must Have)
1. P2P WebRTC connection tests
2. MQTT bridge integration tests
3. HTTP bridge integration tests
4. BUNDLE comprehensive tests
5. Conflict resolution tests
6. Stream signal type tests
7. Encryption tests
8. Authentication tests
9. Security penetration tests

### 🟠 High Priority (Should Have)
1. TCP transport tests
2. Error handling expansion
3. Load test expansion
4. Python binding tests

### 🟡 Medium Priority (Nice to Have)
1. WebSocket bridge tests
2. Socket.IO bridge tests
3. sACN bridge tests
4. DMX bridge tests
5. QUERY tests
6. ANNOUNCE tests
7. Lock/unlock tests
8. Network edge case tests
9. Race condition tests
10. WASM tests
11. Real-world scenario tests

### 🟢 Low Priority (Future)
1. BLE transport tests
2. Serial transport tests
3. Soak test expansion
4. JavaScript/TypeScript additional tests
5. Rust additional tests

---

## Part 10: Timeline Estimate

### Phase 1: Critical Tests (2-3 weeks)
- P2P tests
- MQTT/HTTP bridge tests
- BUNDLE/Stream tests
- Security tests

### Phase 2: High Priority Tests (2-3 weeks)
- TCP transport tests
- Error handling expansion
- Python binding tests

### Phase 3: Medium Priority Tests (3-4 weeks)
- Remaining bridge tests
- Advanced feature tests
- Edge case tests

### Phase 4: Low Priority Tests (Ongoing)
- BLE/Serial tests
- Soak test expansion
- Additional binding tests

**Total Estimate:** 7-10 weeks for critical and high priority, ongoing for medium/low priority

---

## Part 11: Success Criteria

### Test Coverage
- [ ] All critical tests implemented
- [ ] All high priority tests implemented
- [ ] 80%+ code coverage for core protocol
- [ ] 70%+ code coverage for bridges
- [ ] All tests passing in CI/CD

### Test Quality
- [ ] Tests are deterministic (no flakiness)
- [ ] Tests are fast (< 5 minutes for full suite)
- [ ] Tests are well-documented
- [ ] Tests cover edge cases
- [ ] Tests verify both success and failure paths

### Documentation
- [ ] Test execution documented
- [ ] Test purpose documented
- [ ] Coverage reports generated
- [ ] Test results published

---

**Last Updated:** January 23, 2026  
**Status:** 📋 Planning complete, ready for implementation
