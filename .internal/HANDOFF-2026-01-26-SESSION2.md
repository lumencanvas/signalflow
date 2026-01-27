# Session Handoff - 2026-01-26 (Session 2)

**Version:** v3.3.0
**Focus:** P2P API, Rendezvous Integration, Release

## Summary

Extended clasp-client P2P API, integrated rendezvous server into relay, published v3.3.0 to all registries.

## What Was Done

### 1. P2P Client API Extensions
- `send_p2p(peer_session_id, data, reliable)` - Send data to peers
- `set_p2p_routing_mode(mode)` - Control routing (PreferP2P/P2POnly/ServerOnly)
- `p2p_routing_mode()` - Get current mode
- Re-exported `SendResult`, `RoutingMode` in lib.rs
- Fixed P2P data reception (wired `on_data` callback in WebRtcTransport)
- Added connection timeout with `P2PEvent::ConnectionFailed`

### 2. P2P Tests Added
- Test 6: `test_p2p_data_transfer()` - Data flows over P2P
- Test 7: `test_p2p_routing_mode()` - Routing modes work
- Test 8: `test_p2p_nonexistent_peer()` - Timeout handling

### 3. Rendezvous Integration
- Integrated rendezvous server into `deploy/relay`
- Runs on port 7340 by default (alongside WebSocket on 7330)
- CLI flags: `--rendezvous-port`, `--rendezvous-ttl`
- Updated Dockerfile and docker-compose.yml
- Updated all examples to use `https://relay.clasp.to`

### 4. v3.3.0 Release
Published to:
- crates.io: clasp-core, clasp-transport, clasp-discovery, clasp-bridge, clasp-router, clasp-client, clasp-embedded, clasp-wasm, clasp-cli
- npm: @clasp-to/core
- PyPI: clasp-to
- GitHub release + tag v3.3.0

## Files Modified

```
crates/clasp-client/src/client.rs      # P2P methods
crates/clasp-client/src/lib.rs         # Re-exports
crates/clasp-client/src/p2p.rs         # Data callbacks, timeout
crates/clasp-transport/src/webrtc.rs   # on_data callback
clasp-e2e/src/bin/p2p_connection_tests.rs  # Tests 6,7,8
deploy/relay/Cargo.toml                # clasp-discovery dep
deploy/relay/src/main.rs               # Rendezvous startup
deploy/relay/Dockerfile                # Port 7340
deploy/relay/docker-compose.yml        # Port 7340
deploy/relay/.gitignore                # Created
examples/*/p2p*.{js,py,rs}             # relay.clasp.to URLs
docs/api/common/discovery.md           # Relay integration
crates/clasp-discovery/README.md       # Updated docs
site/package-lock.json                 # 3.3.0
```

## Relay Architecture

| Port | Service |
|------|---------|
| 7330 | WebSocket (CLASP) |
| 7340 | Rendezvous HTTP API |
| 7331 | QUIC (optional) |
| 1883 | MQTT (optional) |

## Deployment

DigitalOcean auto-deploys on push. For rendezvous to work at `relay.clasp.to/api/v1/*`, need either:
1. Expose port 7340 separately
2. Configure reverse proxy to route `/api/v1/*` to port 7340

## CI Status

All passing.

## Next Steps

1. Deploy relay with rendezvous enabled
2. Configure proxy for rendezvous API
3. Test end-to-end WAN discovery

---

**Status:** Complete
