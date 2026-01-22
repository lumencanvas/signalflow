# CLASP Hardening Plan

**Goal:** Make CLASP's claims defensible, its benchmarks meaningful, and its implementation production-ready.

This document addresses the legitimate criticisms that:
1. Current benchmarks only measure codec speed, not system throughput
2. Claims need asterisks and honest conditions
3. Missing stress tests for realistic topologies
4. Router implementation may have contention issues at scale

---

## Part 1: Audit Current Claims

### 1.1 Files with performance claims to update

| File | Claim | Status | Action |
|------|-------|--------|--------|
| `README.md` | "8M encode, 11M decode" | ⚠️ Misleading | Add "codec-only" caveat |
| `CLASP-Protocol-v3.md` | "5x faster", "microsecond scheduling" | ⚠️ Aspirational | Clarify conditions |
| `site/src/components/SpecSection.vue` | Performance table | ⚠️ No context | Add methodology note |
| `HANDOFF.md` | Wire protocol summary | ✅ OK | Already technical |

### 1.2 Claims that need rewording

**Before:**
> "CLASP v3 achieves 8M msg/s encoding, 11M msg/s decoding"

**After:**
> "CLASP v3 codec achieves 8M msg/s encoding, 11M msg/s decoding in isolated benchmarks (single core, in-memory, no routing/state/fanout). System throughput depends on topology and features enabled."

**Before:**
> "Microsecond-level deterministic scheduling"

**After:**
> "NTP-style clock sync targeting ±1ms on LAN, ±5-10ms on WiFi. Not suitable for hard realtime or safety-critical applications."

---

## Part 2: Real Benchmark Suite

### 2.1 Benchmark Matrix

Create `test-suite/src/bin/real_benchmarks.rs` with these scenarios:

#### Scenario A: End-to-End Single Hop
```
Publisher → Router → Subscriber (1 sub)
Transport: WebSocket / UDP / QUIC
Metrics: msgs/s, p50/p95/p99 latency, loss %
```

#### Scenario B: Fanout Curve  
```
1 Publisher → Router → N Subscribers
N = 1, 10, 50, 100, 500, 1000
Metrics: throughput vs N, latency degradation curve
```

#### Scenario C: Address Table Scale
```
Router with K registered addresses
K = 100, 1k, 10k, 100k
Metrics: wildcard match time, memory usage
```

#### Scenario D: Wildcard Routing Cost
```
Pattern complexity vs throughput:
- Exact: /lights/kitchen/brightness
- Single wildcard: /lights/*/brightness  
- Globstar: /lights/**
- Complex: /lights/*/zone/*/brightness
```

#### Scenario E: Feature Toggle Matrix
```
| Feature | Off | On | Delta |
|---------|-----|-----|-------|
| State (param cache) | X msg/s | Y msg/s | -Z% |
| Late-joiner replay | X msg/s | Y msg/s | -Z% |
| Scheduling (future bundles) | X msg/s | Y msg/s | -Z% |
| Encryption (TLS) | X msg/s | Y msg/s | -Z% |
| Compression (LZ4) | X msg/s | Y msg/s | -Z% |
```

#### Scenario F: Bridge Overhead
```
CLASP → OSC Bridge → OSC client
CLASP → MIDI Bridge → MIDI device
CLASP → MQTT Bridge → MQTT broker
Metrics: added latency, throughput ceiling
```

### 2.2 Benchmark Implementation

```rust
// test-suite/src/bin/real_benchmarks.rs

/// Scenario B: Fanout curve
async fn benchmark_fanout_curve() {
    let subscriber_counts = [1, 10, 50, 100, 500, 1000];
    
    for n in subscriber_counts {
        let router = TestRouter::start().await;
        let mut subscribers = Vec::with_capacity(n);
        
        // Create N subscribers
        for i in 0..n {
            let client = Clasp::connect_to(&router.url()).await?;
            client.subscribe("/bench/**", |_, _| {}).await?;
            subscribers.push(client);
        }
        
        // Publisher sends burst
        let publisher = Clasp::connect_to(&router.url()).await?;
        let start = Instant::now();
        let msg_count = 10_000;
        
        for i in 0..msg_count {
            publisher.set("/bench/value", i as f64).await?;
        }
        
        let elapsed = start.elapsed();
        let throughput = msg_count as f64 / elapsed.as_secs_f64();
        let total_deliveries = msg_count * n;
        
        println!("N={}: {} msg/s sent, {} deliveries/s fanout", 
                 n, throughput, total_deliveries as f64 / elapsed.as_secs_f64());
    }
}
```

---

## Part 3: Router Implementation Audit

### 3.1 Hot Path Analysis

Identify and optimize these critical paths:

```rust
// Current router flow (pseudocode):
fn handle_set(msg: SetMessage) {
    // 1. Validate address
    // 2. Check permissions (if auth enabled)
    // 3. Update state store (lock?)
    // 4. Match subscriptions (O(n) or O(log n)?)
    // 5. Clone message for each subscriber (Arc clone?)
    // 6. Send to each subscriber (channel send?)
}
```

### 3.2 Contention Points to Audit

| Component | Potential Issue | Check |
|-----------|-----------------|-------|
| State store | `RwLock<HashMap>` contention | Use `dashmap` or sharded locks |
| Subscription registry | Linear scan for wildcard match | Use trie or radix tree |
| Message fanout | Arc clone per subscriber | Consider zero-copy with `Bytes` |
| Async runtime | Tokio task spawn per message | Batch or use channels |
| Atomic counters | Revision increment contention | Use thread-local or sharded |

### 3.3 Specific Code Locations to Review

```
crates/clasp-router/src/router.rs    - Main routing logic
crates/clasp-router/src/state.rs     - State storage
crates/clasp-router/src/subs.rs      - Subscription management
crates/clasp-core/src/address.rs     - Wildcard matching
```

---

## Part 4: Stress Test Suite

### 4.1 Scale Tests

```rust
// test-suite/src/bin/stress_tests.rs

/// 10k addresses, measure memory and lookup time
async fn test_address_scale_10k() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url()).await?;
    
    // Register 10k unique addresses
    for i in 0..10_000 {
        client.set(&format!("/addr/{}/value", i), i as f64).await?;
    }
    
    // Measure wildcard query time
    let start = Instant::now();
    let results = client.query("/addr/*/value").await?;
    let query_time = start.elapsed();
    
    assert_eq!(results.len(), 10_000);
    println!("10k address wildcard query: {:?}", query_time);
}

/// 1000 subscribers, measure fanout latency
async fn test_subscriber_scale_1000() {
    let router = TestRouter::start().await;
    let received = Arc::new(AtomicU64::new(0));
    
    // Create 1000 subscribers
    let mut subscribers = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let client = Clasp::connect_to(&router.url()).await?;
        let counter = received.clone();
        client.subscribe("/fanout/**", move |_, _| {
            counter.fetch_add(1, Ordering::Relaxed);
        }).await?;
        subscribers.push(client);
    }
    
    // Send one message, should fan out to all 1000
    let sender = Clasp::connect_to(&router.url()).await?;
    let start = Instant::now();
    sender.set("/fanout/value", 1.0).await?;
    
    // Wait for all deliveries
    while received.load(Ordering::Relaxed) < 1000 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    let fanout_time = start.elapsed();
    println!("1000 subscriber fanout: {:?}", fanout_time);
}

/// Late-joiner replay storm
async fn test_late_joiner_replay_storm() {
    let router = TestRouter::start().await;
    let setter = Clasp::connect_to(&router.url()).await?;
    
    // Pre-populate 1000 params
    for i in 0..1000 {
        setter.set(&format!("/state/{}", i), i as f64).await?;
    }
    
    // New client subscribes - should get SNAPSHOT with 1000 values
    let start = Instant::now();
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    let late_joiner = Clasp::connect_to(&router.url()).await?;
    late_joiner.subscribe("/state/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await?;
    
    // Wait for snapshot
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let replay_time = start.elapsed();
    let count = received.load(Ordering::Relaxed);
    println!("Late-joiner received {} params in {:?}", count, replay_time);
}

/// Scheduled bundle cascade
async fn test_scheduled_bundle_cascade() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url()).await?;
    
    // Schedule 100 bundles, each 10ms apart
    let base_time = client.server_time().await? + 100_000; // 100ms from now
    
    for i in 0..100 {
        let bundle = BundleMessage {
            timestamp: Some(base_time + i * 10_000), // 10ms apart
            messages: vec![
                Message::Set(SetMessage {
                    address: format!("/scheduled/{}", i),
                    value: Value::Float(i as f64),
                    revision: None,
                    lock: false,
                    unlock: false,
                }),
            ],
        };
        client.send(Message::Bundle(bundle)).await?;
    }
    
    // Wait and verify execution order
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Check that all values were set
    for i in 0..100 {
        let val = client.get(&format!("/scheduled/{}", i)).await?;
        assert_eq!(val.as_f64(), Some(i as f64));
    }
}
```

### 4.2 Failure Mode Tests

```rust
/// Test behavior under overload
async fn test_backpressure_behavior() {
    let router = TestRouter::start().await;
    let sender = Clasp::connect_to(&router.url()).await?;
    
    // Send 100k messages as fast as possible
    let start = Instant::now();
    let mut sent = 0u64;
    let mut errors = 0u64;
    
    for i in 0..100_000 {
        match sender.set("/flood/value", i as f64).await {
            Ok(_) => sent += 1,
            Err(_) => errors += 1,
        }
    }
    
    let elapsed = start.elapsed();
    println!("Sent {} in {:?} ({} errors)", sent, elapsed, errors);
    println!("Effective rate: {} msg/s", sent as f64 / elapsed.as_secs_f64());
}

/// Test clock sync accuracy
async fn test_clock_sync_accuracy() {
    let router = TestRouter::start().await;
    let client = Clasp::connect_to(&router.url()).await?;
    
    // Measure clock offset multiple times
    let mut offsets = Vec::with_capacity(100);
    
    for _ in 0..100 {
        let t1 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        let server_time = client.server_time().await?;
        
        let t2 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        let rtt = t2 - t1;
        let estimated_server_time = t1 + rtt / 2;
        let offset = (server_time as i64) - (estimated_server_time as i64);
        
        offsets.push(offset);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let mean_offset: f64 = offsets.iter().map(|&x| x as f64).sum::<f64>() / offsets.len() as f64;
    let variance: f64 = offsets.iter().map(|&x| (x as f64 - mean_offset).powi(2)).sum::<f64>() / offsets.len() as f64;
    let std_dev = variance.sqrt();
    
    println!("Clock sync: mean offset = {:.0}µs, std dev = {:.0}µs", mean_offset, std_dev);
}
```

---

## Part 5: Documentation Updates

### 5.1 Performance Section Rewrite

**README.md** new performance section:

```markdown
## Performance

### Codec Benchmarks (In-Memory)

These measure raw encode/decode speed on a single core, no networking:

| Protocol | Encode | Decode | Size |
|----------|--------|--------|------|
| MQTT | 11.4M/s | 11.4M/s | 19 B |
| **CLASP v3** | **8M/s** | **11M/s** | **31 B** |
| OSC | 4.5M/s | 5.7M/s | 24 B |

### System Benchmarks (End-to-End)

Real-world throughput with routing, state, and fanout:

| Scenario | Throughput | p99 Latency |
|----------|------------|-------------|
| 1 pub → 1 sub (no state) | TBD | TBD |
| 1 pub → 100 subs (fanout) | TBD | TBD |
| 1 pub → 1 sub (with state) | TBD | TBD |
| Wildcard routing (10k addrs) | TBD | TBD |

### Timing Guarantees

- **LAN (wired):** Target ±1ms clock sync
- **WiFi:** Target ±5-10ms clock sync
- **Not suitable for:** Hard realtime, safety-critical, industrial control

### What These Numbers Mean

- **Codec speed** = theoretical ceiling, useful for comparing wire format efficiency
- **System throughput** = what you'll actually see in production
- **Expect 10-100x reduction** from codec speed to system throughput depending on features enabled
```

### 5.2 New "Honest Limitations" Section

Add to `CLASP-Protocol-v3.md`:

```markdown
## Limitations & Non-Goals

### What CLASP Is NOT

1. **Not hard realtime** — CLASP is soft realtime for creative applications (VJ, lighting, music). It does not provide bounded worst-case latency guarantees.

2. **Not industrial control** — Do not use for safety-critical systems, robotics requiring µs precision, or systems where failure causes physical harm.

3. **Not a replacement for DDS/ROS2** — Those systems provide formal QoS contracts and deterministic delivery. CLASP trades formalism for simplicity.

### Known Limitations

1. **Clock sync is best-effort** — NTP-style sync works well on LAN but degrades on WiFi, NAT, and relay paths. "Microsecond scheduling" is a target, not a guarantee.

2. **Wildcard routing scales O(n)** — With 100k+ addresses, wildcard matching becomes expensive. Use exact addresses in hot paths.

3. **State replay can storm** — Late-joining clients with broad subscriptions may receive large snapshots. Design address hierarchies accordingly.

4. **Bridge timing is inherited** — MIDI has ~1ms jitter, DMX is 44Hz, OSC is UDP best-effort. CLASP cannot make bridges faster than their underlying protocols.
```

---

## Part 6: Implementation Priorities

### Phase 1: Measure (1 week)
- [ ] Implement `real_benchmarks.rs` with Scenarios A-F
- [ ] Run and document baseline numbers
- [ ] Identify actual bottlenecks (not assumed ones)

### Phase 2: Fix Claims (2 days)
- [ ] Update README.md with caveats
- [ ] Update CLASP-Protocol-v3.md with limitations section  
- [ ] Update site/SpecSection.vue with methodology notes

### Phase 3: Optimize Bottlenecks (2 weeks)
Based on Phase 1 findings:
- [ ] If subscription matching is slow → implement trie
- [ ] If state contention is high → use dashmap
- [ ] If fanout is slow → optimize channel usage
- [ ] If memory grows → audit allocations

### Phase 4: Stress Tests (1 week)
- [ ] 10k address scale test
- [ ] 1000 subscriber fanout test
- [ ] Late-joiner replay storm test
- [ ] Scheduled bundle cascade test
- [ ] Backpressure behavior test
- [ ] Clock sync accuracy test

### Phase 5: Security Hardening (1 week)
- [ ] Add replay protection (nonce/timestamp window)
- [ ] Add audit logging for mutations
- [ ] Review bridge sandboxing
- [ ] Document security model honestly

---

## Part 7: Success Criteria

### Benchmarks
- [ ] End-to-end throughput documented with real numbers
- [ ] Fanout curve published (1 to 1000 subscribers)
- [ ] Wildcard routing cost quantified
- [ ] Feature toggle overhead measured

### Documentation
- [ ] All performance claims have methodology notes
- [ ] Limitations section is honest and complete
- [ ] "Codec speed ≠ system throughput" is clear

### Tests
- [ ] All stress tests pass
- [ ] CI runs scale tests (even if abbreviated)
- [ ] Failure modes are documented and tested

### Code Quality
- [ ] Router hot path audited for contention
- [ ] No Arc clone in message fanout (use Bytes)
- [ ] Subscription matching is O(log n) not O(n)

---

## Appendix: Benchmark Methodology Template

For each benchmark, document:

```yaml
name: "Fanout to 100 subscribers"
topology: 1 publisher → router → 100 subscribers
transport: WebSocket (localhost)
message_size: 31 bytes (SET with f64)
message_count: 10,000
features_enabled:
  - state: true
  - wildcards: false
  - scheduling: false
  - encryption: false
hardware: 
  cpu: Apple M1 Pro
  ram: 16GB
  os: macOS 14.0
results:
  throughput: X msg/s
  latency_p50: Y ms
  latency_p95: Z ms
  latency_p99: W ms
  memory_peak: V MB
```

This ensures benchmarks are reproducible and comparable.
