# Complete Architecture Summary

**Date:** 2026-01-22  
**Status:** Finalized Model  
**Decision:** Protocol-Centric Organization

---

## The Decision

After deep analysis of user mental models, code architecture, and UI/UX principles, we've determined:

**✅ Protocol-Centric Organization is the Best Model**

**Why:**
- Matches how users think ("I need OSC" not "I need an OSC server")
- Simpler mental model (one section, not three)
- More flexible (multiple connections per protocol)
- Role is configuration, not organization
- Clear distinction: Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)

---

## The Model

### Core Structure

```
┌─────────────────────────────────────────────────────────┐
│  CLASP ROUTERS                                          │
│  (Central message hub)                                  │
│  - Can enable/disable transports (WebSocket, QUIC, TCP) │
│  - Settings on router, not separate components          │
└─────────────────────────────────────────────────────────┘
                    ▲
                    │
┌───────────────────┴───────────────────────────────────┐
│  PROTOCOL CONNECTIONS                                   │
│  (Connect protocols to CLASP)                          │
│  - Organized by Protocol (OSC, MIDI, MQTT, etc.)       │
│  - Role (server/client/device) is a configuration      │
│  - Bidirectional (one connection handles both)         │
│  - Always connects to CLASP router                     │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  DIRECT CONNECTIONS                                     │
│  (Protocol-to-Protocol, bypasses CLASP)                 │
│  - For advanced users                                  │
│  - Direct protocol-to-protocol                         │
│  - No CLASP translation                                 │
└─────────────────────────────────────────────────────────┘
```

### Key Concepts

1. **CLASP Router**
   - Central message hub
   - Routes messages between clients
   - Manages state, subscriptions, sessions
   - Transports are settings (WebSocket, QUIC, TCP)

2. **Protocol Connection**
   - Bidirectional translator between external protocol and CLASP
   - Organized by protocol (OSC, MIDI, MQTT, etc.)
   - Role (server/client/device) is a configuration setting
   - Always connects to CLASP router
   - One connection handles both directions

3. **Direct Bridge**
   - Protocol-to-protocol connection
   - Bypasses CLASP router
   - For advanced users
   - Direct translation between protocols

---

## Terminology

### Primary Terms

| Term | Use When | Example |
|------|----------|---------|
| **CLASP Router** | The central message hub | "Start a CLASP router" |
| **Protocol Connection** | Protocol adapter connected to CLASP | "OSC Connection to CLASP Router" |
| **Direct Bridge** | Protocol-to-protocol (bypasses CLASP) | "OSC → MIDI direct bridge" |
| **Saved Destination** | Saved config for mappings/tests | "Save destination for mappings" |

### Secondary Terms (Configuration)

- **Connection Type:** Server / Client / Device Interface
- **Status:** "→ Connected to: CLASP Router"

---

## What Each Component Does

### CLASP Router
- ✅ Actually starts a CLASP router (spawns `clasp-router` process)
- ✅ Standalone server that listens on port
- ✅ Accepts CLASP protocol connections
- ✅ Routes messages between clients
- ✅ Manages state, subscriptions, sessions
- ✅ Transports are settings (WebSocket, QUIC, TCP)

### Protocol Connections
- ✅ **Bidirectional** translators between external protocols and CLASP
- ✅ Listen for external protocol messages → translate to CLASP
- ✅ Receive CLASP messages → translate to external protocol
- ✅ **Always connect to CLASP router** (as WebSocket client)
- ✅ Role with external protocol varies:
  - OSC: Server (binds to UDP port)
  - MQTT: Client (connects to broker)
  - MIDI: Device Interface (opens MIDI ports)
  - WebSocket: Server or Client (user chooses)

### Direct Bridges
- ✅ Explicit protocol-to-protocol connections
- ✅ Bypass CLASP router
- ✅ For advanced users
- ✅ Direct translation between protocols

---

## UI Structure

### Sidebar

```
CLASP ROUTERS
  - Router 1 @ localhost:7330
  - Router 2 @ localhost:7331
  [+ ADD ROUTER]

PROTOCOL CONNECTIONS
  - OSC Connection (Server on 9000) → Router 1
  - OSC Connection (Server on 8000) → Router 1
  - MQTT Connection (Client to broker) → Router 1
  - MIDI Connection (Device: Launchpad) → Router 1
  [+ ADD PROTOCOL]

DIRECT CONNECTIONS
  - OSC → MIDI Bridge
  [+ CREATE DIRECT BRIDGE]
```

### Modal Flow

**"ADD PROTOCOL" Modal:**
1. Select protocol (OSC, MIDI, MQTT, etc.)
2. Configure connection:
   - Connection type (server/client/device) - shown as setting
   - Protocol-specific settings
   - Connect to CLASP Router (dropdown)
3. Create connection

---

## Implementation Status

### ✅ Completed Analysis
- [x] Architecture analysis complete
- [x] User mental model analysis complete
- [x] Component capabilities mapped
- [x] Protocol adapter roles clarified
- [x] Protocol-centric model determined
- [x] Master consolidation plan created
- [x] Implementation roadmap created

### 🚧 Ready for Implementation
- [ ] Fix "internal" router connection (CRITICAL)
- [ ] Separate routers from protocol connections
- [ ] Make router selection explicit
- [ ] Reorganize UI sidebar
- [ ] Update modal flow
- [ ] Update terminology
- [ ] Update all documentation

---

## File Update Summary

### Critical Files (Must Update)

1. **`apps/bridge/electron/main.js`**
   - Fix "internal" router connection
   - Implement signal forwarding to router
   - Add router selection logic

2. **`apps/bridge/src/app.js`**
   - Separate `state.routers` from `state.servers`
   - Update `handleAddServer()` → `handleAddProtocol()`
   - Add router selection
   - Show connection status

3. **`apps/bridge/src/index.html`**
   - Reorganize sidebar sections
   - Update modal flow
   - Update all terminology

### Documentation Files (Must Update)

1. **`README.md`**
   - Protocol connections terminology
   - Updated examples
   - Architecture diagram

2. **`docs/guides/bridge-setup.md`**
   - Protocol connections explanation
   - Updated desktop app instructions

3. **`docs/guides/desktop-app-servers.md`**
   - Complete rewrite for protocol-centric model

4. **`crates/clasp-cli/README.md`**
   - Protocol connections terminology
   - Updated examples

---

## Key Insights

### 1. Users Think in Protocols
- "I need OSC" not "I need an OSC server"
- Protocol-first organization is intuitive

### 2. Role is Configuration
- Server/client/device is a setting within protocol selection
- Not the primary way to organize

### 3. Bidirectional Adapters
- One connection handles both directions
- No need for separate input/output

### 4. Clear Distinction
- Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)
- Users understand the difference

### 5. Transports are Router Settings
- WebSocket, QUIC, TCP are settings on router
- Not separate components

---

## Related Documents

### Master Documents
- **`.internal/MASTER-CONSOLIDATION-PLAN.md`** - **START HERE** - Comprehensive file-by-file update plan
- **`.internal/IMPLEMENTATION-ROADMAP.md`** - Phase-by-phase implementation plan
- **`.internal/PLAN-HANDOFF.md`** - Master plan with all tasks

### Analysis Documents
- **`.internal/DEEP-UI-ARCHITECTURE-ANALYSIS.md`** - Deep UI/UX analysis
- **`.internal/COMPONENT-CAPABILITIES-MAP.md`** - What each component does
- **`.internal/PROTOCOL-ADAPTER-ROLES.md`** - Server vs Client roles
- **`.internal/ARCHITECTURE-FINDINGS-AND-RECOMMENDATIONS.md`** - Complete analysis
- **`.internal/ACTUAL-ARCHITECTURE-MAP.md`** - What each component actually does

---

## Next Steps

1. **Review consolidation plan** - Confirm approach
2. **Start Phase 1** - Fix critical issues
3. **Implement UI changes** - Phase 2
4. **Update documentation** - Phase 3
5. **Test with users** - Get feedback
6. **Iterate** - Refine based on feedback

---

*This is the definitive summary. All other documents reference this model.*
