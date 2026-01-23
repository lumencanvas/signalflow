# Master Consolidation Plan: Protocol-Centric Model

**Date:** 2026-01-22  
**Status:** Comprehensive Update Plan  
**Model:** Protocol-Centric Organization (not role-based)

---

## Executive Summary

After deep analysis, we've determined that the **protocol-centric model** is the best approach:
- **Organize by Protocol** (OSC, MIDI, MQTT, etc.) - matches user mental model
- **Role as Configuration** (server/client/device) - a setting, not primary organization
- **Clear Distinction:** Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)

This document consolidates all findings and provides specific update recommendations for every file.

---

## Core Model: Protocol-Centric Architecture

### Key Principles

1. **Users think in protocols, not roles**
   - "I need OSC" not "I need an OSC server"
   - "I need MIDI" not "I need a MIDI client"

2. **Role is configuration, not organization**
   - Server/client/device is a setting within protocol selection
   - Not the primary way to organize the UI

3. **Clear separation:**
   - **Protocol Connections** → Connect protocols to CLASP router
   - **Direct Bridges** → Protocol-to-protocol (bypass CLASP)

4. **Bidirectional adapters**
   - One adapter handles both directions
   - No need for separate input/output adapters

---

## Terminology Updates

### Primary Terms

| Current | New | Context |
|---------|-----|---------|
| "ADD SERVER" | "ADD PROTOCOL" | Button/modal title |
| "CLASP Server" | "CLASP Router" | Router component |
| "OSC Server" | "OSC Connection" | When connected to CLASP |
| "OSC Server" | "OSC Server" | When standalone (future) |
| "Protocol Bridges" | "Protocol-to-Protocol Bridges" or "Direct Connections" | Direct protocol connections |
| "OUTPUT TARGETS" | "Saved Destinations" | Saved configs, not active |

### Secondary Terms (Show in UI, Not Primary)

- **Connection Type:** Server / Client / Device Interface
- **Status:** "→ Connected to: CLASP Router" or "Standalone (no CLASP)"

---

## File-by-File Update Plan

### 1. Desktop App (`apps/bridge/`)

#### `apps/bridge/src/index.html`

**Current Issues:**
- Button: "ADD SERVER"
- Modal title: "ADD SERVER"
- Sidebar: "MY SERVERS"

**Updates Needed:**
- [ ] Change button: "ADD SERVER" → "ADD PROTOCOL"
- [ ] Change modal title: "ADD SERVER" → "ADD PROTOCOL CONNECTION"
- [ ] Change sidebar section: "MY SERVERS" → "PROTOCOL CONNECTIONS"
- [ ] Add new section: "CLASP ROUTERS" (separate from protocol connections)
- [ ] Add new section: "DIRECT CONNECTIONS" (for protocol-to-protocol bridges)
- [ ] Update help text: "Connect [protocol] to CLASP router"
- [ ] Add connection status display: "→ Connected to: CLASP Router"
- [ ] Update "OUTPUT TARGETS" → "SAVED DESTINATIONS"

**Modal Flow:**
1. Select protocol (OSC, MIDI, MQTT, etc.)
2. Configure connection:
   - Connection type (server/client/device) - shown as setting
   - Protocol-specific settings
   - Connect to CLASP Router (dropdown)
3. Create connection

#### `apps/bridge/src/app.js`

**Current Issues:**
- `state.servers` mixes CLASP routers and protocol adapters
- `handleAddServer()` creates bridges but doesn't show them
- No explicit router selection

**Updates Needed:**
- [ ] Separate `state.routers` from `state.protocolConnections`
- [ ] Rename `state.servers` → `state.protocolConnections` (or keep servers but clarify)
- [ ] Update `handleAddServer()` → `handleAddProtocol()`
- [ ] Add router selection dropdown in protocol connection modal
- [ ] Show connection status: "→ Connected to: [Router Name]"
- [ ] Add auto-created bridges to `state.bridges` with `autoCreated: true`
- [ ] Implement actual forwarding of bridge signals to CLASP router (fix "internal" issue)
- [ ] Update `renderServers()` → `renderProtocolConnections()`
- [ ] Add `renderRouters()` for CLASP routers section
- [ ] Add `renderDirectConnections()` for protocol-to-protocol bridges

#### `apps/bridge/electron/main.js`

**Current Issues:**
- `startOscServer()` creates bridge with `target_addr: 'internal'` but doesn't forward signals
- No explicit router connection logic

**Updates Needed:**
- [ ] Fix "internal" router connection - actually forward signals to CLASP router
- [ ] When `target_addr: 'internal'`, find first running CLASP router
- [ ] Create WebSocket connection to router
- [ ] Forward all bridge signals to router
- [ ] Store router connection per bridge
- [ ] Return bridge info when creating protocol connection
- [ ] Add router selection to protocol connection creation

---

### 2. Main README (`README.md`)

**Current Issues:**
- Uses "server" terminology inconsistently
- Doesn't clearly explain protocol connections vs routers
- CLI examples could be clearer

**Updates Needed:**
- [ ] Update Quick Start section:
  ```bash
  # 1. Start CLASP router (required)
  clasp server --port 7330
  
  # 2. Add protocol connections (connect to router)
  clasp osc --port 9000  # Creates OSC connection to router
  clasp mqtt --host localhost --port 1883  # Creates MQTT connection to router
  ```
- [ ] Clarify: "Bridge commands create protocol connections that connect to CLASP router"
- [ ] Update architecture diagram to show "Protocol Connections" instead of "Bridges"
- [ ] Add note: "Protocol connections are bidirectional - one connection handles both directions"
- [ ] Update terminology: "CLASP Server" → "CLASP Router" where appropriate
- [ ] Clarify transports are router settings, not separate components

**Specific Lines to Update:**
- Line 86: "Start a router first, then add bridges" → "Start a router first, then add protocol connections"
- Line 94: "OSC bridge: listens for OSC, translates to CLASP" → "OSC connection: listens for OSC, connects to router"
- Line 106: Update note about bridge commands

---

### 3. Documentation Index (`docs/index.md`)

**Current Issues:**
- Uses "server" terminology
- Doesn't reflect protocol-centric model

**Updates Needed:**
- [ ] Update description: "Protocol Connections" instead of "Protocol Bridges"
- [ ] Update architecture diagram
- [ ] Update terminology throughout

---

### 4. Bridge Setup Guide (`docs/guides/bridge-setup.md`)

**Current Issues:**
- Uses "bridge" terminology but should clarify "protocol connection"
- Desktop app instructions say "ADD SERVER" (needs update)
- Doesn't clearly explain protocol-centric model

**Updates Needed:**
- [ ] Update title/description to clarify: "Protocol Connections" (connect to CLASP) vs "Direct Bridges" (bypass CLASP)
- [ ] Update Desktop App section:
  - "ADD SERVER" → "ADD PROTOCOL"
  - Explain: "Creates a protocol connection that connects to CLASP router"
  - Show connection status
- [ ] Add section: "Understanding Protocol Connections"
  - Explain bidirectional nature
  - Explain connection to router
  - Show visual flow
- [ ] Update all "bridge" references to "protocol connection" when referring to CLASP connections
- [ ] Keep "bridge" for direct protocol-to-protocol connections

**Specific Sections:**
- Line 38: Update "add a server" → "add a protocol connection"
- Line 109: Update "ADD SERVER" → "ADD PROTOCOL"
- Line 113: Clarify it creates a "protocol connection" not just a "bridge"

---

### 5. Desktop App Servers Guide (`docs/guides/desktop-app-servers.md`)

**Current Issues:**
- Entire document needs rewrite for protocol-centric model
- Uses "ADD SERVER" terminology
- Doesn't explain protocol connections clearly

**Updates Needed:**
- [ ] Rewrite with protocol-centric model
- [ ] Title: "Desktop App: Understanding Protocol Connections"
- [ ] Explain: "ADD PROTOCOL" creates protocol connections
- [ ] Show new UI structure:
  - CLASP ROUTERS section
  - PROTOCOL CONNECTIONS section
  - DIRECT CONNECTIONS section
- [ ] Explain bidirectional nature
- [ ] Show connection status
- [ ] Update all terminology

---

### 6. CLI README (`crates/clasp-cli/README.md`)

**Current Issues:**
- Uses "bridge" terminology
- Doesn't clearly explain protocol connections

**Updates Needed:**
- [ ] Update "Start Protocol Bridges" → "Start Protocol Connections"
- [ ] Clarify: "These commands create protocol connections that connect to CLASP router"
- [ ] Update examples to show connection to router
- [ ] Add note about bidirectional nature

**Specific Lines:**
- Line 35: "Start Protocol Bridges" → "Start Protocol Connections"
- Line 36: Update note about bridges
- Line 38: "Start an OSC bridge" → "Start an OSC connection"

---

### 7. Protocol Mapping Guide (`docs/guides/protocol-mapping.md`)

**Current Issues:**
- Should be fine, but verify terminology

**Updates Needed:**
- [ ] Verify all examples use correct terminology
- [ ] Add note about bidirectional protocol connections
- [ ] Clarify protocol-to-protocol vs protocol-to-CLASP

---

### 8. Architecture Documentation (`docs/architecture.md`)

**Current Issues:**
- May need updates for protocol-centric model

**Updates Needed:**
- [ ] Review and update diagrams
- [ ] Clarify protocol connections vs direct bridges
- [ ] Update terminology

---

### 9. Protocol-Specific Docs (`docs/protocols/*.md`)

**Current Issues:**
- May use inconsistent terminology

**Updates Needed:**
- [ ] Review each protocol doc
- [ ] Ensure consistent terminology
- [ ] Clarify connection types (server/client/device)
- [ ] Add examples showing connection to CLASP router

---

### 10. Website Copy (`site/`)

**Current Issues:**
- Need to review all website content

**Updates Needed:**
- [ ] Review all pages
- [ ] Update terminology: "CLASP Server" → "CLASP Router"
- [ ] Update: "Protocol Bridges" → "Protocol Connections" (when referring to CLASP)
- [ ] Add explanation of protocol-centric model
- [ ] Update screenshots when UI changes are made
- [ ] Update examples

---

## Implementation Checklist

### Phase 1: Critical Fixes (Must Do First)

- [ ] **Fix "internal" router connection**
  - Implement actual forwarding of bridge signals to CLASP router
  - Files: `apps/bridge/electron/main.js`
  
- [ ] **Separate routers from protocol connections**
  - Create `state.routers` array
  - Separate CLASP routers from protocol connections
  - Files: `apps/bridge/src/app.js`

- [ ] **Make router selection explicit**
  - Add dropdown in protocol connection modal
  - Show which router each connection uses
  - Files: `apps/bridge/src/app.js`, `apps/bridge/src/index.html`

### Phase 2: UI Reorganization (High Priority)

- [ ] **Reorganize sidebar**
  - CLASP ROUTERS section
  - PROTOCOL CONNECTIONS section
  - DIRECT CONNECTIONS section
  - Files: `apps/bridge/src/index.html`, `apps/bridge/src/app.js`

- [ ] **Update modal flow**
  - "ADD PROTOCOL" button
  - Select protocol first
  - Configure role as setting
  - Files: `apps/bridge/src/index.html`, `apps/bridge/src/app.js`

- [ ] **Update terminology in UI**
  - "ADD SERVER" → "ADD PROTOCOL"
  - "CLASP Server" → "CLASP Router"
  - "OSC Server" → "OSC Connection"
  - Files: All UI files

### Phase 3: Documentation Updates (High Priority)

- [ ] **Update README.md**
  - Protocol-centric terminology
  - Clear examples
  - Architecture diagram

- [ ] **Update docs/guides/bridge-setup.md**
  - Protocol connections explanation
  - Updated desktop app instructions
  - Clear distinction: connections vs direct bridges

- [ ] **Rewrite docs/guides/desktop-app-servers.md**
  - Protocol-centric model
  - New UI structure
  - Connection status

- [ ] **Update crates/clasp-cli/README.md**
  - Protocol connections terminology
  - Clear examples

### Phase 4: Additional Documentation (Medium Priority)

- [ ] **Review and update all protocol docs**
  - Consistent terminology
  - Connection examples

- [ ] **Update architecture.md**
  - Protocol-centric diagrams
  - Clear explanations

- [ ] **Review website copy**
  - Update all terminology
  - Update examples
  - Update screenshots

### Phase 5: Advanced Features (Lower Priority)

- [ ] **Add transport settings to router**
  - WebSocket, QUIC, TCP selection
  - Files: Router creation modal

- [ ] **Improve server scanning**
  - Click to add
  - Rename option
  - Create bridge option

- [ ] **Clarify outputs**
  - Rename to "Saved Destinations"
  - Add help text

---

## New UI Structure (Visual)

```
┌─────────────────────────────────────────────────────────┐
│  SIDEBAR                                                 │
│                                                          │
│  ┌───────────────────────────────────────────────────┐ │
│  │  CLASP ROUTERS                                    │ │
│  │  ┌─────────────────────────────────────────────┐ │ │
│  │  │ CLASP Router @ localhost:7330               │ │ │
│  │  │ [Running] [Edit] [Delete]                   │ │ │
│  │  │ Transports: WebSocket ✅                     │ │ │
│  │  └─────────────────────────────────────────────┘ │ │
│  │  [+ ADD ROUTER]                                  │ │
│  └───────────────────────────────────────────────────┘ │
│                                                          │
│  ┌───────────────────────────────────────────────────┐ │
│  │  PROTOCOL CONNECTIONS                             │ │
│  │  (Connect protocols to CLASP)                     │ │
│  │                                                    │ │
│  │  ┌─────────────────────────────────────────────┐ │ │
│  │  │ OSC Connection                               │ │ │
│  │  │ Server on 0.0.0.0:9000                      │ │ │
│  │  │ → Connected to: CLASP Router                │ │ │
│  │  │ [Running] [Edit] [Delete]                    │ │ │
│  │  └─────────────────────────────────────────────┘ │ │
│  │                                                    │ │
│  │  ┌─────────────────────────────────────────────┐ │ │
│  │  │ MQTT Connection                              │ │ │
│  │  │ Client to localhost:1883                     │ │ │
│  │  │ → Connected to: CLASP Router                │ │ │
│  │  │ [Running] [Edit] [Delete]                    │ │ │
│  │  └─────────────────────────────────────────────┘ │ │
│  │                                                    │ │
│  │  ┌─────────────────────────────────────────────┐ │ │
│  │  │ MIDI Connection                              │ │ │
│  │  │ Device: Launchpad Pro                       │ │ │
│  │  │ → Connected to: CLASP Router                │ │ │
│  │  │ [Running] [Edit] [Delete]                    │ │ │
│  │  └─────────────────────────────────────────────┘ │ │
│  │                                                    │ │
│  │  [+ ADD PROTOCOL]                                 │ │
│  └───────────────────────────────────────────────────┘ │
│                                                          │
│  ┌───────────────────────────────────────────────────┐ │
│  │  DIRECT CONNECTIONS                               │ │
│  │  (Protocol-to-Protocol, bypasses CLASP)          │ │
│  │                                                    │ │
│  │  ┌─────────────────────────────────────────────┐ │ │
│  │  │ OSC → MIDI Bridge                             │ │ │
│  │  │ Direct connection (no CLASP)                  │ │ │
│  │  │ [Running] [Edit] [Delete]                     │ │ │
│  │  └─────────────────────────────────────────────┘ │ │
│  │                                                    │ │
│  │  [+ CREATE DIRECT BRIDGE]                         │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

---

## Terminology Matrix

### When to Use Each Term

| Term | Use When | Example |
|------|----------|---------|
| **CLASP Router** | Referring to the central message hub | "Start a CLASP router" |
| **Protocol Connection** | Protocol adapter connected to CLASP | "OSC Connection to CLASP Router" |
| **Protocol Adapter** | Generic term for protocol translator | "OSC adapter translates OSC to CLASP" |
| **Direct Bridge** | Protocol-to-protocol (bypasses CLASP) | "OSC → MIDI direct bridge" |
| **Protocol-to-Protocol Bridge** | Same as direct bridge | "Create a protocol-to-protocol bridge" |
| **Saved Destination** | Saved config for mappings/tests | "Save destination for use in mappings" |
| **Connection Type** | Server/Client/Device (configuration) | "Connection type: Server" |

---

## User Flow Examples (Updated)

### Example 1: TouchOSC to Lights

**User thinks:** "I want TouchOSC to control my lights"

**Steps:**
1. Add CLASP Router → Starts router on localhost:7330
2. Add Protocol → OSC
   - Connection Type: Server (default)
   - Bind: 0.0.0.0:9000
   - Connect to: CLASP Router @ localhost:7330
3. Add Protocol → Art-Net
   - Connection Type: Server (default)
   - Bind: 0.0.0.0:6454
   - Connect to: CLASP Router @ localhost:7330
4. Create Mapping: OSC `/fader1` → Art-Net U1/C1

**User sees:**
- Routers: CLASP Router @ localhost:7330
- Protocol Connections: OSC Connection → CLASP Router, Art-Net Connection → CLASP Router
- Mappings: OSC `/fader1` → Art-Net U1/C1

**User understands:** Everything flows through CLASP router

### Example 2: MQTT Sensors

**User thinks:** "I want my MQTT sensors to control everything"

**Steps:**
1. Add CLASP Router
2. Add Protocol → MQTT
   - Connection Type: Client (only option)
   - Broker: localhost:1883
   - Topics: sensors/#
   - Connect to: CLASP Router
3. Messages automatically flow to CLASP router
4. Create mappings to route to other protocols

**User sees:**
- Protocol Connections: MQTT Connection (Client to localhost:1883) → CLASP Router

**User understands:** MQTT messages flow to CLASP, then can be routed anywhere

### Example 3: Direct OSC to MIDI (No CLASP)

**User thinks:** "I just want OSC to control MIDI, no CLASP"

**Steps:**
1. Create Direct Bridge
2. Source: OSC (0.0.0.0:9000)
3. Target: MIDI (Launchpad Pro)
4. No CLASP router needed

**User sees:**
- Direct Connections: OSC → MIDI Bridge
- Shows "Direct connection (no CLASP)"

**User understands:** This bypasses CLASP, direct protocol-to-protocol

---

## Benefits of Protocol-Centric Model

1. **Matches User Mental Model**
   - Users think "I need OSC" not "I need an OSC server"
   - Protocol-first organization is intuitive

2. **Clear Organization**
   - All OSC together, all MIDI together
   - Easy to find and manage

3. **Flexible**
   - Multiple connections per protocol
   - Role is configuration, not limitation

4. **Clear Distinction**
   - Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)
   - Users understand the difference

5. **Bidirectional**
   - One connection handles both directions
   - No need for separate input/output

6. **Easy to Understand**
   - Digital artists can follow the flow
   - Clear visual indicators

---

## Migration Path

### For Existing Users

1. **Backward Compatibility**
   - Keep `state.servers` array (rename internally)
   - Map old "servers" to new "protocol connections"
   - Show migration message on first launch

2. **Data Migration**
   - Convert existing server configs to protocol connections
   - Preserve all settings
   - Add router connection info

3. **UI Migration**
   - Show both old and new UI during transition
   - Allow users to switch
   - Eventually remove old UI

---

## Testing Checklist

- [ ] Test protocol connection creation
- [ ] Test router selection
- [ ] Test connection status display
- [ ] Test bidirectional communication
- [ ] Test direct bridges
- [ ] Test with multiple routers
- [ ] Test with multiple connections per protocol
- [ ] Test UI with non-technical users
- [ ] Verify all documentation is updated
- [ ] Verify website copy is updated

---

## Related Documents

- `.internal/DEEP-UI-ARCHITECTURE-ANALYSIS.md` - Detailed UI/UX analysis
- `.internal/COMPONENT-CAPABILITIES-MAP.md` - What each component does
- `.internal/PROTOCOL-ADAPTER-ROLES.md` - Server vs Client roles
- `.internal/ARCHITECTURE-FINDINGS-AND-RECOMMENDATIONS.md` - Complete analysis
- `.internal/PLAN-HANDOFF.md` - Master plan with all tasks

---

## Next Steps

1. **Review this consolidation plan** - Confirm approach
2. **Prioritize implementation** - Start with critical fixes
3. **Implement Phase 1** - Fix "internal" router connection
4. **Implement Phase 2** - Reorganize UI
5. **Update documentation** - Phase 3
6. **Test with users** - Get feedback
7. **Iterate** - Refine based on feedback

---

*This document should be updated as implementation progresses.*
