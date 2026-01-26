# CLASP Codebase Remediation - Handoff

**Date**: 2026-01-25
**Previous Commit**: `4980c38`
**Branch**: `main`

## Summary

Implemented 8 security and stability fixes across the CLASP codebase, addressing issues from CRITICAL to LOW severity. Subsequently addressed 7 additional gaps and improvements identified during codebase analysis.

---

## Session 2: Gap Analysis & Remediation (2026-01-25)

### Phase 6A: Documentation Sync - Security Model
**File**: `.internal/analysis/14-SECURITY-MODEL.md`

- Updated outdated token generation description (was LCG, now UUID v4)
- Fixed token format from "base62" to "hex"
- Updated security considerations to reflect cryptographically secure generation

### Phase 6B: State Cleanup Task Implementation
**File**: `crates/clasp-router/src/router.rs`

- Added `start_state_cleanup_task()` that runs every 60 seconds
- Calls `state.cleanup_stale()` to remove expired params and signals
- Integrated into both `serve_on()` and `serve_all()` methods
- **Fixes potential memory leak** from accumulated stale state entries

### Phase 6C: Python Binding Tests
**File**: `bindings/python/tests/test_client.py`

- Added 15 new tests for `query_signals()`, `gesture()`, `timeline()` methods
- Tests cover: signatures, not-connected error handling, message encoding constants
- Total: 33 Python tests pass

### Phase 6D: Fuzz Corpus Seeds
**Directory**: `crates/clasp-core/fuzz/corpus/`

- Created `fuzz_decode_frame/` with 6 seed files (valid frames with various flags)
- Created `fuzz_decode_message/` with 34 seed files (all message types)
- Seeds include: HELLO, SET, SUBSCRIBE, PUBLISH, PING, PONG, SYNC, ERROR, QUERY, etc.

### Phase 6E: JavaScript Binding Methods
**Files**: `bindings/js/packages/clasp-core/src/client.ts`, `types.ts`

- Added `gesture(address, gestureId, phase, payload)` method with full JSDoc
- Added `timeline(address, keyframes, options)` method with loop and startTime
- Added `TimelineKeyframe` interface
- **Achieves feature parity with Python bindings**
- Total: 41 JS tests pass

### Phase 6F: Language Bindings Documentation
**File**: `.internal/analysis/11-LANGUAGE-BINDINGS.md`

- Updated feature parity table to reflect current state
- Added `gesture()` and `timeline()` rows
- Fixed Signal Queries row with implementation notes

### Phase 6G: Adapter Integration Tests
**File**: `crates/clasp-router/tests/adapter_tests.rs`

- Added 13 tests for MQTT and OSC server adapters
- Tests cover: config defaults, topic/address conversion, value type conversion, shared state
- Feature-gated for `mqtt-server`, `osc-server`, `websocket` features

### Phase 6H: CI/Build Fixes

**Issue**: Build/tests were failing in CI due to two problems:

1. **Formatting violations**: Multiple files had incorrect indentation in `RouterConfig` struct initializations
2. **Clippy `approx_constant` errors**: Test code used `3.14` and `3.14159` which triggers a deny-by-default clippy lint for approximate mathematical constants

**Files fixed**:
- Formatting: Auto-fixed with `cargo fmt --all` (many files in e2e, router, transport)
- Clippy: Replaced `3.14`/`3.14159` with `1.25`/`1.2345` in test files:
  - `crates/clasp-core/benches/codec.rs`
  - `crates/clasp-core/src/codec.rs`
  - `crates/clasp-core/tests/codec_tests.rs`
  - `crates/clasp-core/tests/embedded_compat_tests.rs`
  - `crates/clasp-embedded/src/lib.rs`
  - `crates/clasp-transport/tests/transport_tests.rs`
  - `crates/clasp-router/tests/bundle_tests.rs`
  - `crates/clasp-client/tests/client_tests.rs`
  - `crates/clasp-wasm/tests/web.rs`
  - `clasp-e2e/src/bin/embedded_tests.rs`
  - `clasp-e2e/src/bin/relay_e2e.rs`
  - `clasp-e2e/src/bin/public_relay_tests.rs`
  - `clasp-e2e/src/compliance/encoding.rs`
  - `clasp-e2e/src/compliance/messages.rs`

---

## Session 1: Original Remediation (2026-01-25)

## Changes Made

### Phase 1: CRITICAL - Token Generation
**Files**: `crates/clasp-core/src/security.rs`, `crates/clasp-core/Cargo.toml`

- Replaced weak time-seeded LCG with UUID v4 (uses `getrandom` internally)
- Token format changed from base62 to hex: `cpsk_<32-char-uuid>`
- Added tests: `test_cpsk_token_uniqueness` (10K tokens), `test_cpsk_token_format`

**Clarification**: This change **only affects token generation** (`CpskValidator::generate_token()`).
All capability and scope mechanics remain unchanged:
- Scoped permissions (`read:/path/**`, `write:/lights/*`, `admin:/**`) - unchanged
- Token validation (HashMap lookup by token string) - unchanged
- TokenInfo structure (scopes, expiration, metadata) - unchanged
- Pattern matching for authorization - unchanged
- Existing registered tokens - still valid

### Phase 2A: StateStore Limits
**File**: `crates/clasp-core/src/state.rs`

- Added `StateStoreConfig` with `max_params`, `param_ttl`, `EvictionStrategy`
- Added `last_accessed` timestamp to `ParamState`
- Implemented eviction strategies: `Lru`, `OldestFirst`, `RejectNew`
- Added `cleanup_stale()` and `cleanup_stale_with_config()` methods
- Default: 10,000 params max, 1 hour TTL, LRU eviction
- Backward compatible: `StateStore::new()` uses unlimited config

### Phase 2B: RouterState Cleanup
**Files**: `crates/clasp-router/src/state.rs`, `crates/clasp-router/src/subscription.rs`

- Added `RouterStateConfig` with signal TTL and limits
- Added `SignalEntry` with registration/access timestamps
- Added `cleanup_stale_signals()` method
- Fixed `SubscriptionManager::remove()` to clean up empty `by_prefix` entries
- Fixed `remove_session()` to also clean up prefix index

### Phase 3: Embedded Protocol Compatibility
**File**: `crates/clasp-embedded/src/lib.rs`

- Added message types: `ANNOUNCE`, `BUNDLE`, `SYNC`, `QUERY`, `RESULT`
- Added value type constants: `ARRAY`, `MAP`
- Added `ValueExt` enum with `String`/`Bytes` support (requires `alloc` feature)
- Added `decode_value_ext()` and `encode_value_ext()` functions
- Fixed `encode_subscribe()` format for core compatibility (type_mask + opt_flags)

### Phase 4A: Fuzz Testing
**Files**: `crates/clasp-core/fuzz/`

- Created fuzz testing infrastructure with `cargo-fuzz`
- `fuzz_decode_frame`: Tests `Frame::decode` with random bytes
- `fuzz_decode_message`: Tests `decode_message` with valid headers + random payloads

### Phase 4B: Cross-Implementation Tests
**File**: `crates/clasp-core/tests/embedded_compat_tests.rs`

10 tests verifying embedded ↔ core interoperability:
- SET encoding/decoding both directions
- All value types roundtrip
- HELLO/WELCOME handshake
- PING/PONG, SUBSCRIBE compatibility
- Frame header compatibility
- Edge cases (long addresses, infinity, min/max int)

### Phase 5A: Python Binding Methods
**File**: `bindings/python/python/clasp/client.py`

- Added `query_signals(pattern)` / `get_signals(pattern)`
- Added `gesture(address, gesture_id, phase, payload)`
- Added `timeline(address, keyframes, loop, start_time)`
- Added RESULT message handling in `_handle_message()`
- Added `_pending_queries` dict for async query tracking

### Phase 5B: QoS Degradation Logging
**File**: `crates/clasp-bridge/src/osc.rs`

- Added warning log when QoS is downgraded for CLASP→OSC bridge
- Uses structured logging: `warn!(address, original_qos, "CLASP->OSC: QoS downgraded...")`

## Test Results

```
Total: 495 tests passed, 0 failed, 6 ignored
```

All workspace tests pass including e2e tests.

## Verification Commands

```bash
# Run all tests
cargo test --workspace

# Run specific phase tests
cargo test -p clasp-core test_cpsk           # Phase 1
cargo test -p clasp-core state               # Phase 2A
cargo test -p clasp-router state             # Phase 2B
cargo test -p clasp-embedded                 # Phase 3
cargo test -p clasp-core --test embedded_compat_tests  # Phase 4B

# Verify no_std build
cargo build -p clasp-embedded --no-default-features

# Run fuzz tests (requires nightly)
cd crates/clasp-core/fuzz
cargo +nightly fuzz run fuzz_decode_frame -- -max_total_time=300
cargo +nightly fuzz run fuzz_decode_message -- -max_total_time=300
```

## Breaking Changes

None. All changes are backward compatible:
- `StateStore::new()` uses unlimited config (same as before)
- `RouterState::new()` uses unlimited config (same as before)
- Token format changed but existing tokens remain valid

## Future Considerations

All items from Session 1 have been addressed in Session 2:
- ~~Background cleanup tasks~~ → **Done**: Phase 6B
- ~~Python tests~~ → **Done**: Phase 6C
- ~~Fuzz corpus~~ → **Done**: Phase 6D

### Remaining Items (Low Priority)

1. **MQTT auth validation**: `TODO` at `mqtt_server.rs:360` - validate username/password against token validator
2. **MQTT retained messages**: Not supported in server adapter
3. **MQTT will messages**: Not supported in server adapter
4. **Embedded ACK/GET/SNAPSHOT encoding**: By design, embedded devices are data sources
