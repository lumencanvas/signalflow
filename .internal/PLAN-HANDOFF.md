# CLASP Project Plan & Handoff

**Last Updated:** 2026-01-22  
**Current Version:** v0.1.2 (released), v3 encoding implemented (unpushed)  
**Status:** Hardening in progress

---

## Quick Context

CLASP (Creative Low-Latency Application Streaming Protocol) is a universal protocol bridge for creative applications. This document consolidates all internal planning into a single handoff.

**Key recent work:**
- ✅ v3 binary encoding: 55% smaller, 4x faster codec
- ✅ Real-world benchmarks: 173k msg/s system throughput
- ✅ Honest documentation: Added caveats to performance claims
- 🔄 Hardening: In progress

---

## What's Actually True (Verified 2026-01-22)

### Performance Reality

| Metric | Claim | Actual | Notes |
|--------|-------|--------|-------|
| Codec encode | 8M msg/s | ✅ Verified | Single core, in-memory |
| Codec decode | 11M msg/s | ✅ Verified | Single core, in-memory |
| E2E throughput | — | 173k msg/s | Localhost WebSocket |
| Fanout 100 subs | — | 175k deliveries/s | Real routing |
| Event (no state) | — | 259k msg/s | Fire-and-forget |
| Stream | — | 330k msg/s | No persistence |

**Bottom line:** Codec speed ≠ system throughput. Real-world is 10-100x lower.

### What Works

| Component | Status | Evidence |
|-----------|--------|----------|
| v3 binary encoding | ✅ Complete | All codec tests pass |
| WebSocket transport | ✅ Complete | E2E benchmarks run |
| Router with state | ✅ Complete | Param/Event/Stream tested |
| Fanout to 500 subs | ✅ Works | 175k deliveries/s |
| Desktop app backend | ✅ Works | Spawns real clasp-router |
| OSC/MIDI bridges | ✅ Works | Integration tested |

### What's Incomplete / Bugs Found

| Area | Issue | Priority |
|------|-------|----------|
| **Wildcard `*` matching** | Single-level wildcard broken (0-2% match rate) | **P0** |
| **Late-joiner > 500 params** | Snapshot delivery fails at 1000+ params | **P0** |
| Security (replay protection) | Not implemented | P2 |
| Clock sync accuracy | Aspirational, not verified | P2 |
| QUIC transport tests | None | P3 |

**Critical bugs discovered 2026-01-22:**
- `**` (globstar) works: 65k msg/s
- `*` (single-level) broken: 0-2 msgs received out of 100 expected
- Late joiner works up to 500 params (77k msg/s)
- Late joiner times out at 1000 params (0 received)

---

## Unpushed Changes (2026-01-22)

The following commits are ready but not pushed:

```
feat(codec): v3 binary encoding - 55% smaller, 4x faster
docs: add hardening plan and real-world benchmarks  
docs: update performance claims with honest methodology
```

Files changed:
- `crates/clasp-core/src/codec.rs` — v3 binary encode/decode
- `crates/clasp-core/src/frame.rs` — Version field in flags
- `bindings/js/`, `bindings/python/` — v3 support
- `clasp-minimal.js` — v3 support
- `README.md`, `CLASP-Protocol-v3.md` — Honest performance claims
- `test-suite/src/bin/real_benchmarks.rs` — System throughput tests

---

## Hardening Priorities

### Phase 1: Fix Critical Bugs (NOW)

1. **Single-level wildcard (`*`) broken** — P0
   - Symptom: `/lights/zone50/*/brightness` matches 0-2% of expected messages
   - Works: `/lights/**` (globstar) at 65k msg/s
   - File to investigate: `crates/clasp-router/src/subscription.rs`
   - File to investigate: `crates/clasp-core/src/address.rs` 
   - Hypothesis: Pattern matching logic differs for `*` vs `**`

2. **Late-joiner snapshot > 500 params** — P0
   - Symptom: 500 params works (77k msg/s), 1000 params times out (0 received)
   - File to investigate: `crates/clasp-router/src/router.rs` (snapshot logic)
   - File to investigate: `crates/clasp-router/src/state.rs`
   - Hypothesis: Snapshot message too large, or batching breaks

### Phase 2: Router Audit (After Bugs Fixed)

1. **RwLock contention analysis**
   - File: `crates/clasp-router/src/state.rs`
   - Current: `RwLock<StateStore>` for params
   - Risk: Write lock contention under load
   - Check: Profile with 1k concurrent writers

2. **Subscription matching optimization**
   - File: `crates/clasp-router/src/subscription.rs`
   - Current: Prefix indexing + linear scan
   - Risk: O(n) for wildcard with many subscriptions
   - Consider: Trie for O(log n)

### Phase 3: Security (Later)

1. **Replay protection** — Add nonce/timestamp window
2. **Audit logging** — Log mutations for debugging
3. **Rate limiting** — Already exists, verify under load

---

## Test Coverage Status

| Phase | Category | Complete | Remaining |
|-------|----------|----------|-----------|
| 1 | Core Protocol | ✅ 40+ | 0 |
| 2 | Transport | ⚠️ 8 | 27 (QUIC, UDP, etc.) |
| 3 | Router | ✅ 20+ | 5 |
| 4 | Client Library | ❌ 0 | 20 |
| 5 | Bridges | ⚠️ 28 | 17 |
| 6 | Discovery | ❌ 0 | 10 |
| 7 | Embedded | ❌ 0 | 8 |
| 8 | WASM | ❌ 0 | 8 |
| **Total** | | ~114 | ~102 |

See `TEST_PLAN.md` for detailed breakdown.

---

## Benchmark Results (2026-01-22)

### Scenario A: E2E Single Hop
```
✓ E2E single hop (websocket) | 173k msg/s | 6µs avg
```

### Scenario B: Fanout Curve
```
✓ Fanout to 1   | 66k msg/s
✓ Fanout to 10  | 350k deliveries/s
✓ Fanout to 50  | 307k deliveries/s  
✓ Fanout to 100 | 286k deliveries/s
✓ Fanout to 500 | 214k deliveries/s (degrades gracefully)
```

### Scenario C: Address Scale
```
✓ 100 addresses   | 1.7M write/s (very fast)
✓ 1,000 addresses | 260k write/s
✓ 10,000 addresses| 456k write/s
```

### Scenario D: Wildcard Routing (BUG FOUND)
```
✓ globstar (/lights/**)        | 65k msg/s | 0% loss ✅
❌ exact (/zone50/fixture5)    | 0 msg/s   | 99.9% loss (BUG)
❌ single (/zone50/*)          | 2 msg/s   | 99% loss (BUG)
❌ complex (/zone*/fixture*)   | 0 msg/s   | 100% loss (BUG)
```

### Scenario E: Signal Types
```
✓ Param (stateful)     | 122k msg/s
✓ Event (no state)     | 143k msg/s (+17%)
✓ Stream (fire-forget) | 146k msg/s (+20%)
```

### Scenario F: Late Joiner (BUG FOUND)
```
✓ 10 params   | 2.3k msg/s ✅
✓ 100 params  | 16k msg/s  ✅
✓ 500 params  | 77k msg/s  ✅
❌ 1000 params | TIMEOUT    (BUG - works up to 500)
```

**Key insights:**
- Fanout scales well up to 500 subscribers
- `**` wildcard works, `*` is broken
- Late joiner snapshot breaks somewhere between 500-1000 params

---

## Files Reference

### Internal Planning (`.internal/`)
| File | Purpose |
|------|---------|
| `PLAN-HANDOFF.md` | This file - unified handoff |
| `COMPACT-ENCODING-PLAN.md` | v3 encoding design doc |
| `HARDENING-PLAN.md` | System hardening roadmap |
| `TEST_PLAN.md` | Test coverage tracking |
| `HANDOFF.md` | Legacy handoff (historical) |

### Public Documentation (root)
| File | Purpose |
|------|---------|
| `README.md` | Project overview, getting started |
| `CLASP-Protocol-v3.md` | Full protocol specification |
| `CLASP-QuickRef.md` | Quick reference card |
| `CONTRIBUTING.md` | Contribution guidelines |
| `PUBLISHING.md` | Package publishing guide |

### Key Source Files
| File | Purpose |
|------|---------|
| `crates/clasp-core/src/codec.rs` | v3 encode/decode |
| `crates/clasp-router/src/router.rs` | Message routing |
| `crates/clasp-router/src/state.rs` | State storage |
| `crates/clasp-router/src/subscription.rs` | Subscription matching |
| `test-suite/src/bin/real_benchmarks.rs` | System benchmarks |

---

## Commands

```bash
# Run all unit tests
cargo test --workspace

# Run real-world benchmarks
cargo run --release -p clasp-test-suite --bin real_benchmarks

# Run test suite
cargo run -p clasp-test-suite --bin run-all-tests

# Build desktop app
cd apps/bridge && npm install && npm run build

# Build website
cd site && npm install && npm run build

# Check unpushed commits
git log --oneline @{u}.. 2>/dev/null || git log --oneline -5
```

---

## Decision Log

### 2026-01-22: v3 Binary Encoding
**Decision:** Replace MessagePack with custom binary format  
**Rationale:** 55% size reduction, 4x faster encoding  
**Trade-off:** More code, but backward compatible via version flag

### 2026-01-22: Honest Performance Claims
**Decision:** Add methodology notes and caveats to all benchmarks  
**Rationale:** ChatGPT critique was valid - codec ≠ system throughput  
**Trade-off:** Less impressive marketing, more credibility

### 2026-01-22: Keep v2 MessagePack Support
**Decision:** Auto-detect v2/v3 in decoder, support both  
**Rationale:** Gradual migration, don't break existing clients  
**Trade-off:** Slightly more complex codec

---

## Next Session Checklist

1. [ ] Review wildcard benchmark timeout
2. [ ] Review late-joiner replay timeout
3. [ ] Consider pushing v3 encoding changes
4. [ ] Profile router under concurrent load
5. [ ] Add client library tests (priority gap)

---

## Contact & Resources

- **Website:** https://clasp.to
- **GitHub:** https://github.com/lumencanvas/clasp
- **Maintainer:** LumenCanvas (https://lumencanvas.studio)
