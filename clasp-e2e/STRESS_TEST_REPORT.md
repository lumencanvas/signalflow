# CLASP Relay Stress Test Report

**Target:** `wss://relay.clasp.to` (DigitalOcean deployment)
**Date:** 2026-01-25
**Test Suite:** `relay-stress-tests` (release build)

## Executive Summary

The stress testing revealed **critical issues** with the production relay:

1. **Server crashed/became unresponsive** after ~2 minutes of sustained load testing
2. **Subscription pattern bug** - globstar `/**` patterns fail to match
3. **Payload size limit** - 64KB+ payloads silently fail
4. **Connection resets** - frequent under load without graceful handling

---

## Test Results Before Server Became Unresponsive

### Latency Benchmarks

| Metric | SET→ACK | PUB→SUB |
|--------|---------|---------|
| Min | 91.26ms | 97.98ms |
| **p50** | **96.06ms** | **103.68ms** |
| p95 | 117.18ms | 113.28ms |
| **p99** | **129.22ms** | **115.65ms** |
| Max | 131.84ms | 116.16ms |
| Mean | 98.82ms | 104.37ms |

**Assessment:** Latencies are reasonable for a WAN relay but higher than local (~100ms RTT).

### Throughput Benchmarks

| Test | Result |
|------|--------|
| Single client | 407 msg/s, 100% ACK |
| Fanout (10 subs × 100 msgs) | 1000/1000 delivered (100%) |

**Assessment:** Good throughput with perfect delivery in controlled tests.

### Concurrency Stress

| Test | Result |
|------|--------|
| 100 concurrent clients | 100/100 success (100%) |
| 10 writers × 100 writes to same address | 1000/1000 ACKs |
| 50 rapid connect/disconnect cycles | 50/50 success |

**Assessment:** Excellent concurrency handling.

### Protocol Correctness

| Test | Result |
|------|--------|
| Subscription patterns | **FAILED** - `/**` globstar broken |
| Message ordering | PASSED - 0 out-of-order |
| State consistency | PASSED - late joiner correct |
| Bundle atomicity | PASSED - 3/3 delivered |

### Edge Cases

| Test | Result |
|------|--------|
| 1KB payload | OK (117ms) |
| 4KB payload | OK (102ms) |
| 16KB payload | OK (108ms) |
| 64KB payload | **SEND FAILED** |
| 256KB payload | **SEND FAILED** |
| Special characters | All OK (dashes, underscores, unicode) |
| Null/empty values | All 7 types OK |

### Sustained Load

| Metric | Value |
|--------|-------|
| Duration | 30 seconds |
| Messages sent | 8,243 |
| Throughput | 275 msg/s |
| Delivery rate | 99.6% |

**Assessment:** Good sustained throughput with near-perfect delivery.

---

## Critical Issues Found

### 1. SERVER CRASH/UNRESPONSIVENESS (CRITICAL)

After running the stress test suite (~2 minutes of testing), the server became completely unresponsive:
- WebSocket connections time out
- No graceful degradation
- No rate limiting feedback
- Server appears to have crashed or entered a bad state

**Impact:** Production outage risk under load.

**Recommendation:**
- Implement connection rate limiting with backpressure
- Add circuit breakers
- Monitor and auto-restart on failure
- Consider horizontal scaling

### 2. SUBSCRIPTION GLOBSTAR BUG (HIGH)

The `/**` globstar pattern does not match addresses correctly:

```
Pattern: /stress-test/.../patterns/**
Address: /stress-test/.../patterns/a
Expected: MATCH
Actual: NO MATCH
```

**Impact:** Core subscription functionality broken.

**Location:** Likely in `clasp-router/src/subscription.rs` pattern matching logic.

### 3. PAYLOAD SIZE LIMIT (MEDIUM)

Messages with payloads >16KB fail silently:
- 16KB: Works
- 64KB: SEND FAILED
- 256KB: SEND FAILED

**Impact:** Large data (images, audio, config files) cannot be transmitted.

**Recommendation:**
- Document the limit explicitly
- Return proper error instead of silent failure
- Consider chunking support

### 4. CONNECTION RESETS UNDER LOAD (MEDIUM)

Frequent errors:
```
WebSocket protocol error: Connection reset without closing handshake
```

**Impact:** Clients experience unexpected disconnections.

**Recommendation:**
- Implement graceful shutdown
- Add connection draining
- Improve WebSocket close handling

---

## Test Files Created

1. `clasp-e2e/src/bin/public_relay_tests.rs` - Basic functionality tests
2. `clasp-e2e/src/bin/relay_stress_tests.rs` - Comprehensive stress tests

**Run commands:**
```bash
# Basic tests
cargo run --bin public-relay-tests

# Stress tests (release build recommended)
cargo run --bin relay-stress-tests --release

# With P2P
cargo run --bin relay-stress-tests --release --features p2p
```

---

## Recommendations for Production Readiness

### Immediate (P0)
1. Fix globstar `/**` subscription pattern matching
2. Implement rate limiting and backpressure
3. Add server health monitoring and auto-recovery

### Short-term (P1)
1. Document and handle payload size limits properly
2. Implement graceful connection shutdown
3. Add connection draining during shutdown
4. Implement circuit breakers

### Long-term (P2)
1. Horizontal scaling support
2. Message chunking for large payloads
3. Connection pooling
4. Geographic distribution (CDN/edge)

---

## Conclusion

The CLASP relay has good baseline functionality but **is not production-ready** for high-load scenarios. The server crashed under stress testing, and there's a critical subscription pattern bug. These issues should be addressed before considering the relay production-grade.
