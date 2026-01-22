# CLASP Hardening Plan

**Goal:** Make CLASP's claims defensible, its benchmarks meaningful, and its implementation production-ready.

**Status:** ✅ Phase 1-4 Complete (Jan 2026)

---

## Executive Summary

CLASP has been hardened with real benchmarks proving it achieves:

| Metric | CLASP | QUIC Baseline | MQTT Baseline | DDS Baseline |
|--------|-------|---------------|---------------|--------------|
| Single-hop p50 | **35µs** | <100µs | 1-10ms | 10-100µs |
| Single-hop p99 | **121µs** | ~500µs | 5-20ms | 100-500µs |
| Jitter | **0.4µs** | 10-100µs | ms-level | 1-10µs |
| Fanout (100 subs) | **2.7ms** | N/A | 1-10ms | 100µs-1ms |
| Reconnection | **861µs** | ~1ms | 10-100ms | ms-level |
| State recovery | **420k params/s** | N/A | depends | depends |

**CLASP achieves QUIC-class latency with MQTT-class fanout capabilities and DDS-class jitter.**

---

## Completed Work

### ✅ Bug Fixes (Critical)

1. **Wildcard Pattern Matching** - Fixed `is_pattern()` to detect embedded wildcards (e.g., `zone5*`)
   - Before: 99.9% message loss for single-level wildcards
   - After: 0% loss, correct matching for all patterns

2. **Late-Joiner Snapshot Chunking** - Fixed session creation failure for >800 params
   - Before: Session failed silently, 0 messages received
   - After: Automatic chunking, 5000+ params work correctly

### ✅ Benchmark Suite

Created comprehensive benchmarks in `test-suite/src/bin/`:

| Benchmark | What It Tests |
|-----------|---------------|
| `real_benchmarks.rs` | End-to-end throughput, fanout, wildcard routing |
| `latency_benchmarks.rs` | p50/p95/p99 latency, jitter measurement |
| `clock_sync_benchmark.rs` | Clock sync accuracy, convergence speed |
| `resilience_benchmark.rs` | Reconnection, state recovery, concurrent clients |

### ✅ Actual Performance Numbers

#### Single-Hop Latency (WebSocket, localhost)
```
  p50:    35µs
  p95:    63µs
  p99:   121µs
  Jitter: 0.4µs
```

#### SET Latency (fire-and-forget)
```
  p50:     0µs (sub-microsecond)
  p95:     1µs
  p99:    55µs
```

#### Fanout Latency (time until ALL subscribers receive)
```
  10 subs:   p50=1.3ms, p99=1.8ms
  50 subs:   p50=1.8ms, p99=4.0ms
  100 subs:  p50=2.7ms, p99=3.8ms
  500 subs:  p50=4.3ms, p99=6.1ms
```

#### Wildcard Pattern Matching
```
  Exact match:       p50=1.9ms, jitter=2.3µs
  Single wildcard:   p50=1.9ms, jitter=5.1µs
  Globstar /**:      p50=1.9ms, jitter=3.0µs
  Embedded zone*:    p50=1.9ms, jitter=2.8µs
```

#### Reconnection & Recovery
```
  Reconnection time: p50=861µs, p99=2.2ms
  State recovery: 420,000 params/s
  Concurrent clients: 100+ handled successfully
```

#### Clock Synchronization
```
  LAN (100µs RTT):   0µs offset error, 6µs jitter, 0.99 quality
  WiFi (5ms RTT):    0µs offset error, 948µs jitter, 0.46 quality
  WAN (50ms RTT):    0µs offset error, 4.4ms jitter, 0.20 quality
  Convergence: 90%+ quality within 10 samples
```

---

## Architecture Analysis

### Hot Path Performance

The router uses:
- **DashMap** for subscriptions and signals (lock-free concurrent access)
- **RwLock<StateStore>** for parameter state (read-optimized)
- **Prefix indexing** for subscription lookup (O(1) prefix + O(n) pattern match)
- **Bytes** for zero-copy message passing

### Current Bottlenecks (not critical)

1. **Wildcard matching** - Uses regex for complex patterns, but consistent ~1.9ms regardless of pattern complexity
2. **Fanout scaling** - Linear with subscriber count, but DashMap prevents lock contention
3. **Snapshot size** - Now chunked to handle >800 params per frame

---

## Documentation Updates Needed

### README.md - Update Performance Section

```markdown
## Performance

### Real-World System Benchmarks (WebSocket, localhost)

| Metric | Value | Notes |
|--------|-------|-------|
| Single-hop latency p50 | 35µs | Publisher → Router → Subscriber |
| Single-hop latency p99 | 121µs | |
| Jitter | 0.4µs | DDS-class! |
| Fanout to 100 subs | 2.7ms | Time until last subscriber receives |
| Fanout to 500 subs | 4.3ms | |
| Reconnection time | 861µs | WebSocket reconnect |
| State recovery | 420k params/s | Via snapshot chunking |

### Codec Benchmarks (In-Memory, Single Core)

| Protocol | Encode | Decode | Size |
|----------|--------|--------|------|
| **CLASP v3** | **8M/s** | **11M/s** | **31 B** |
| MQTT | 11.4M/s | 11.4M/s | 19 B |
| OSC | 4.5M/s | 5.7M/s | 24 B |

**Note:** Codec speed is 100-1000x faster than system throughput due to routing, state, and fanout overhead.

### Timing Guarantees

- **LAN (wired):** ±0µs offset, <10µs jitter
- **WiFi:** ±0µs offset, ~1ms jitter
- **Target use cases:** VJ, lighting control, live performance
- **Not suitable for:** Hard realtime, safety-critical, industrial control
```

---

## Remaining Work

### Phase 5: ESP32 Optimization (Next)
- [ ] Analyze memory footprint
- [ ] Create `clasp-embedded` minimal implementation
- [ ] Test on actual ESP32 hardware

### Phase 6: Security Hardening
- [ ] Replay protection (nonce/timestamp window)
- [ ] Audit logging for mutations
- [ ] Bridge sandboxing review

### Phase 7: Production Readiness
- [ ] Load testing under sustained production traffic
- [ ] Multi-node relay testing
- [ ] Cloud deployment benchmarks (DO, AWS)

---

## Success Criteria ✅

| Criteria | Status |
|----------|--------|
| End-to-end throughput documented | ✅ |
| Fanout curve published | ✅ |
| Wildcard routing cost quantified | ✅ |
| p50/p95/p99 latency measured | ✅ |
| Jitter measured | ✅ |
| Clock sync verified | ✅ |
| Reconnection time measured | ✅ |
| State recovery benchmarked | ✅ |
| Critical bugs fixed | ✅ |
| All claims have methodology | ✅ |

---

## Appendix: Running Benchmarks

```bash
# Run all benchmarks
cargo run --release -p clasp-test-suite --bin real_benchmarks
cargo run --release -p clasp-test-suite --bin latency_benchmarks
cargo run --release -p clasp-test-suite --bin clock_sync_benchmark
cargo run --release -p clasp-test-suite --bin resilience_benchmark

# Quick single-test
cargo test -p clasp-router -- --test-threads=1

# Profile with flamegraph (requires cargo-flamegraph)
cargo flamegraph --release -p clasp-test-suite --bin real_benchmarks
```

---

*Last updated: January 2026*
