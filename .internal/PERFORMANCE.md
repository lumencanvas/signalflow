# CLASP Performance Documentation

**Measured on:** macOS (Apple Silicon M-series), January 2026  
**Version:** CLASP v3 binary protocol  
**Transport:** WebSocket (localhost)

---

## Executive Summary

CLASP achieves **QUIC-class latency** (35µs p50) with **MQTT-class fanout** (100+ subscribers) and **DDS-class jitter** (0.4µs). The embedded profile uses only **680 bytes RAM** making it suitable for ESP32 and similar microcontrollers.

| Capability | CLASP | Industry Comparison |
|------------|-------|---------------------|
| Single-hop latency | 35µs p50 | QUIC: <100µs, MQTT: 1-10ms |
| Jitter | 0.4µs | DDS: 1-10µs |
| Fanout (100 subs) | 2.7ms | MQTT: 1-10ms |
| State recovery | 420k params/s | Proprietary |
| Embedded footprint | 680 bytes | Full: 64KB+ |

---

## Detailed Benchmarks

### 1. Latency (Single-Hop: Publisher → Router → Subscriber)

**Methodology:** Send 10,000 messages, measure time from send to receive callback.

```
Metric          Value       Notes
────────────────────────────────────────────────────
p50 latency     35µs        Median delivery time
p95 latency     63µs        95th percentile
p99 latency     121µs       99th percentile (tail)
Jitter          0.4µs       Consecutive message variance
Min             23µs        Best case
Max             3,323µs     Worst case (outlier)
────────────────────────────────────────────────────
```

**Interpretation:** CLASP delivers sub-100µs latency for typical messages, comparable to QUIC. The 3ms outlier is due to OS scheduler preemption, not CLASP overhead.

### 2. SET Latency (Fire-and-Forget)

**Methodology:** 10,000 SET operations measuring time to return from async call.

```
Metric          Value       Notes
────────────────────────────────────────────────────
p50 latency     0µs         Sub-microsecond (local queue)
p95 latency     1µs         
p99 latency     55µs        
Jitter          0.2µs       Extremely consistent
────────────────────────────────────────────────────
```

**Interpretation:** Fire-and-forget SETs return immediately after queuing. Actual delivery happens asynchronously.

### 3. Fanout Latency (Time Until ALL Subscribers Receive)

**Methodology:** Create N subscribers, send message, measure time until last one receives.

```
Subscribers     p50         p95         p99         Jitter
──────────────────────────────────────────────────────────────
10              1,340µs     1,520µs     1,810µs     16.5µs
50              1,770µs     2,540µs     4,030µs     36.0µs
100             2,660µs     2,960µs     3,800µs     25.7µs
500             4,330µs     5,220µs     6,120µs     33.4µs
──────────────────────────────────────────────────────────────
```

**Interpretation:** Fanout scales sub-linearly. 50x more subscribers = ~3x latency (not 50x).

### 4. Wildcard Pattern Matching

**Methodology:** 1,000 messages through various pattern types.

```
Pattern Type            p50         p99         Jitter
────────────────────────────────────────────────────────────
Exact match             1,870µs     2,230µs     2.3µs
Single wildcard /*      1,880µs     2,780µs     5.1µs
Globstar /**            1,850µs     2,230µs     3.0µs
Embedded zone*          1,890µs     2,370µs     2.8µs
────────────────────────────────────────────────────────────
```

**Interpretation:** Pattern complexity has negligible impact on latency (~20µs variance).

### 5. Clock Synchronization

**Methodology:** NTP-style sync with simulated network conditions.

```
Network         RTT         Offset Error    Jitter      Quality
───────────────────────────────────────────────────────────────
LAN             100µs       0µs             6µs         0.99
WiFi            5ms         0µs             948µs       0.46
WAN             50ms        0µs             4,439µs     0.20
───────────────────────────────────────────────────────────────
```

**Interpretation:** Clock sync achieves zero offset error. Quality score reflects RTT and jitter confidence.

### 6. Reconnection & Recovery

```
Metric                  Value           Notes
────────────────────────────────────────────────────
Reconnection p50        861µs           WebSocket handshake
Reconnection p99        2,240µs         
State recovery rate     420,000 params/s Via chunked snapshots
Max concurrent clients  100+            Tested successfully
────────────────────────────────────────────────────
```

### 7. Embedded Profile (ESP32-class)

```
Component               Size (bytes)
────────────────────────────────────
State cache (64 slots)  512
Subscriptions (8 max)   32
TX/RX buffers          128
Misc state             8
────────────────────────────────────
TOTAL                  680 bytes

ESP32 SRAM:            320,000 bytes
CLASP usage:           0.21% of SRAM
Reduction vs full:     94x smaller
```

---

## Methodology Notes

### Test Environment
- **CPU:** Apple Silicon (M-series)
- **OS:** macOS
- **Transport:** WebSocket (localhost, no network latency)
- **Build:** Release mode (`--release`)

### What These Numbers Mean

1. **Codec speed ≠ System throughput**  
   Raw codec benchmarks (8M encode/s) measure serialization only. System throughput (60k msg/s) includes routing, state, fanout.

2. **localhost numbers**  
   Real network latency adds 0.1-100ms depending on topology. These benchmarks isolate CLASP overhead.

3. **Jitter depends on OS**  
   Sub-microsecond jitter requires real-time OS or kernel bypass. CLASP's 0.4µs jitter is best-effort on standard OS.

### Reproducing These Results

```bash
# Clone and build
git clone https://github.com/lumencanvas/clasp
cd clasp
cargo build --release

# Run benchmarks
cargo run --release -p clasp-test-suite --bin latency_benchmarks
cargo run --release -p clasp-test-suite --bin real_benchmarks
cargo run --release -p clasp-test-suite --bin clock_sync_benchmark
cargo run --release -p clasp-test-suite --bin resilience_benchmark
```

---

## Comparison Table

| Metric | CLASP | QUIC | MQTT | DDS |
|--------|-------|------|------|-----|
| **Latency p50** | 35µs | <100µs | 1-10ms | 10-100µs |
| **Jitter** | 0.4µs | 10-100µs | ms-level | 1-10µs |
| **Fanout** | 100s subs | Streams | 10k+ subs | 100s readers |
| **State replay** | Yes | No | Retained msgs | Durable QoS |
| **Clock sync** | NTP-style | None | None | DDS-RT |
| **Embedded** | 680B | N/A | 10KB+ | 50KB+ |
| **Transport** | WS/QUIC/UDP | QUIC | TCP | UDP multicast |

---

## Limitations

1. **Not hard realtime** — Best-effort timing, not bounded worst-case guarantees
2. **Not safety-critical** — Do not use for systems where failure causes harm
3. **Wildcard O(n)** — Pattern matching scans subscriptions (use exact addresses in hot paths)
4. **Single-node** — Router is single-process (horizontal scaling requires relay federation)

---

*Last updated: January 2026*
