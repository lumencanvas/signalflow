# Quick Start Checklist - Production Readiness

**Created:** 2026-01-23  
**Use this to start working immediately**

---

## Immediate Next Steps (Start Here)

### Step 1: Investigation Phase
- [ ] **INV-001:** Investigate Gesture signal type
  - [ ] Check `crates/clasp-core/src/codec.rs` - verify gesture encode/decode works
  - [ ] Check `crates/clasp-router/src/router.rs` - verify gesture handling exists
  - [ ] Check `crates/clasp-core/src/types.rs` - verify GesturePhase enum complete
  - [ ] Document findings in `.internal/IMPLEMENTATION-TRACKING.md`

- [ ] **INV-002:** Investigate Timeline signal type
  - [ ] Check `crates/clasp-core/src/codec.rs` - verify timeline message structure
  - [ ] Check `crates/clasp-router/src/router.rs` - verify timeline handling exists
  - [ ] Check `crates/clasp-core/src/types.rs` - verify Timeline types complete
  - [ ] Document findings in `.internal/IMPLEMENTATION-TRACKING.md`

- [ ] **INV-003:** Investigate TCP transport
  - [ ] Search codebase for TCP transport implementation
  - [ ] Check if feature-gated or truly missing
  - [ ] Document findings in `.internal/IMPLEMENTATION-TRACKING.md`

### Step 2: Start Implementation (After Investigation)

**Gesture Signal Type:**
- [ ] Read CLASP-Protocol.md §4.5 Gesture specification
- [ ] Implement gesture ID tracking in router state
- [ ] Implement gesture phase coalescing
- [ ] Implement gesture lifecycle management
- [ ] Write tests

**Timeline Signal Type:**
- [ ] Read CLASP-Protocol.md §4.6 Timeline specification
- [ ] Design timeline message structure
- [ ] Implement timeline codec
- [ ] Implement timeline storage
- [ ] Implement timeline execution engine
- [ ] Write tests

**TCP Transport (if needed):**
- [ ] Design TCP transport architecture
- [ ] Implement TCP server
- [ ] Implement TCP client
- [ ] Add to router
- [ ] Write tests

### Step 3: Testing Phase

**For each feature:**
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Write end-to-end tests
- [ ] Verify protocol compliance
- [ ] Update documentation

---

## File Locations Reference

### Core Files
- `crates/clasp-core/src/types.rs` - Signal types, GesturePhase, Timeline types
- `crates/clasp-core/src/codec.rs` - Encode/decode logic
- `crates/clasp-router/src/router.rs` - Router message handling
- `crates/clasp-router/src/state.rs` - Router state management

### Test Files
- `test-suite/src/bin/gesture_tests.rs` - NEW: Gesture tests
- `test-suite/src/bin/timeline_tests.rs` - NEW: Timeline tests
- `test-suite/src/bin/tcp_tests.rs` - NEW: TCP tests
- `test-suite/src/bin/late_joiner_tests.rs` - Late-joiner tests
- `test-suite/src/bin/clock_sync_tests.rs` - Clock sync tests
- `test-suite/src/bin/bundle_tests.rs` - Bundle tests
- `test-suite/src/bin/qos_tests.rs` - QoS tests

### Documentation
- `CLASP-Protocol.md` - Protocol specification (MUST FOLLOW)
- `.internal/PRODUCTION-READINESS-IMPLEMENTATION-PLAN.md` - Detailed plan
- `.internal/IMPLEMENTATION-TRACKING.md` - Task tracking
- `.internal/PRODUCTION-READINESS-AUDIT.md` - Audit findings

---

## Testing Commands

```bash
# Run all tests
cargo test --workspace

# Run specific test binary
cargo run -p clasp-test-suite --bin gesture_tests
cargo run -p clasp-test-suite --bin timeline_tests

# Run benchmarks
cargo run -p clasp-test-suite --bin real_benchmarks --release

# Check test coverage (if tool installed)
cargo test --workspace -- --test-threads=1
```

---

## Protocol Compliance Checklist

For each feature, verify:
- [ ] Follows CLASP-Protocol.md specification exactly
- [ ] Message encoding matches spec
- [ ] Message decoding matches spec
- [ ] Router behavior matches spec
- [ ] Error handling matches spec
- [ ] Performance meets targets (if applicable)

---

## Progress Tracking

Update `.internal/IMPLEMENTATION-TRACKING.md` as you work:
1. Change status from 🔍 to 📝 when starting
2. Change status from 📝 to 🚧 when implementing
3. Change status from 🚧 to ✅ when complete
4. Add notes about findings, blockers, etc.

---

## Common Issues & Solutions

**Issue:** Feature is partially implemented
- **Solution:** Complete the implementation, don't leave stubs

**Issue:** Tests exist but are incomplete
- **Solution:** Expand tests to cover all cases

**Issue:** Documentation claims feature but it's not implemented
- **Solution:** Either implement it or remove from documentation

**Issue:** Feature is feature-gated
- **Solution:** Document feature gates, ensure they work when enabled

**Issue:** Hardware required for testing
- **Solution:** Use mocks/virtual devices for CI, document hardware requirements

---

## Success Criteria

Before marking a feature complete:
- [ ] Implementation is complete (no stubs)
- [ ] All tests pass
- [ ] Protocol compliance verified
- [ ] Documentation updated
- [ ] Performance validated (if applicable)
- [ ] Status updated in tracking document

---

**Last Updated:** 2026-01-23
