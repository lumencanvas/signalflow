# Implementation Roadmap: Protocol-Centric Model

**Based on:** `.internal/MASTER-CONSOLIDATION-PLAN.md`  
**Status:** Ready for Implementation  
**Priority:** High

---

## Quick Reference

**Model:** Protocol-Centric Organization
- **Organize by Protocol** (OSC, MIDI, MQTT, etc.)
- **Role as Configuration** (server/client/device is a setting)
- **Clear Distinction:** Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)

**Key Changes:**
- "ADD SERVER" → "ADD PROTOCOL"
- "CLASP Server" → "CLASP Router"
- "OSC Server" → "OSC Connection" (when connected to CLASP)
- Sidebar: CLASP ROUTERS | PROTOCOL CONNECTIONS | DIRECT CONNECTIONS

---

## Phase 1: Critical Fixes (Week 1)

### 1.1 Fix "Internal" Router Connection

**Problem:** Protocol connections create bridges with `target_addr: 'internal'` but signals aren't forwarded to CLASP router.

**Files:**
- `apps/bridge/electron/main.js`

**Tasks:**
- [ ] Implement actual forwarding of bridge signals to CLASP router
- [ ] When `target_addr: 'internal'`, find first running CLASP router
- [ ] Create WebSocket connection to router
- [ ] Forward all bridge signals to router
- [ ] Store router connection per bridge
- [ ] Error if no router exists when trying to connect

**Estimated Time:** 2-3 days

---

### 1.2 Separate Routers from Protocol Connections

**Problem:** `state.servers` mixes CLASP routers and protocol adapters.

**Files:**
- `apps/bridge/src/app.js`

**Tasks:**
- [ ] Create `state.routers` array (separate from `state.servers`)
- [ ] Update `handleAddServer()` to route to router or protocol connection handler
- [ ] Update `renderServers()` to handle both routers and connections
- [ ] Add `renderRouters()` function
- [ ] Update storage/loading to handle both

**Estimated Time:** 1-2 days

---

### 1.3 Make Router Selection Explicit

**Problem:** Protocol connections use `target_addr: 'internal'` (magic string).

**Files:**
- `apps/bridge/src/app.js`
- `apps/bridge/src/index.html`
- `apps/bridge/electron/main.js`

**Tasks:**
- [ ] Add router selection dropdown in protocol connection modal
- [ ] Show list of running CLASP routers
- [ ] Store selected router in connection config
- [ ] Show which router each connection uses in list
- [ ] Error if no router exists when trying to connect

**Estimated Time:** 1-2 days

---

## Phase 2: UI Reorganization (Week 2)

### 2.1 Reorganize Sidebar

**Files:**
- `apps/bridge/src/index.html`
- `apps/bridge/src/app.js`

**Tasks:**
- [ ] Create "CLASP ROUTERS" section
- [ ] Create "PROTOCOL CONNECTIONS" section
- [ ] Create "DIRECT CONNECTIONS" section
- [ ] Update CSS/styling for sections
- [ ] Add section headers and dividers

**Estimated Time:** 2-3 days

---

### 2.2 Update Modal Flow

**Files:**
- `apps/bridge/src/index.html`
- `apps/bridge/src/app.js`

**Tasks:**
- [ ] Change button: "ADD SERVER" → "ADD PROTOCOL"
- [ ] Change modal title: "ADD SERVER" → "ADD PROTOCOL CONNECTION"
- [ ] Update modal flow:
  1. Select protocol (OSC, MIDI, MQTT, etc.)
  2. Configure connection:
     - Connection type (server/client/device) - shown as setting
     - Protocol-specific settings
     - Connect to CLASP Router (dropdown)
  3. Create connection
- [ ] Update `handleAddServer()` → `handleAddProtocol()`

**Estimated Time:** 2-3 days

---

### 2.3 Update Terminology in UI

**Files:**
- `apps/bridge/src/index.html`
- `apps/bridge/src/app.js`
- All UI-related files

**Tasks:**
- [ ] "ADD SERVER" → "ADD PROTOCOL" (all instances)
- [ ] "CLASP Server" → "CLASP Router" (all instances)
- [ ] "OSC Server" → "OSC Connection" (when connected to CLASP)
- [ ] "OUTPUT TARGETS" → "SAVED DESTINATIONS"
- [ ] Add connection status: "→ Connected to: CLASP Router"
- [ ] Update all help text and tooltips

**Estimated Time:** 1-2 days

---

### 2.4 Show Auto-Created Bridges

**Files:**
- `apps/bridge/src/app.js`
- `apps/bridge/electron/main.js`

**Tasks:**
- [ ] Add auto-created bridges to `state.bridges` array
- [ ] Mark as `autoCreated: true` and link to protocol connection
- [ ] Show in Bridges tab with "Auto" label
- [ ] Allow user to edit/delete auto-created bridges
- [ ] Update `renderBridges()` to show auto-created section

**Estimated Time:** 1-2 days

---

## Phase 3: Documentation Updates ✅ COMPLETED (2026-01-22)

### 3.1 Update README.md ✅

**Tasks:**
- [x] Update Quick Start section with protocol connections terminology
- [x] Clarify: "Bridge commands create protocol connections that connect to CLASP router"
- [x] Update terminology: "CLASP Server" → "CLASP Router"
- [x] Add note about bidirectional connections

---

### 3.2 Update Bridge Setup Guide ✅

**File:** `docs/guides/bridge-setup.md`

**Tasks:**
- [x] Update Desktop App section: "ADD SERVER" → "ADD PROTOCOL"
- [x] Clarify protocol connections vs direct bridges
- [x] Update all terminology

---

### 3.3 Rewrite Desktop App Servers Guide ✅

**File:** `docs/guides/desktop-app-servers.md`

**Tasks:**
- [x] Rewrite for protocol-centric model
- [x] Title: "Desktop App: Understanding Protocol Connections"
- [x] Update all terminology

---

### 3.4 Update CLI README ✅

**File:** `crates/clasp-cli/README.md`

**Tasks:**
- [x] Update "Start Protocol Bridges" → "Start Protocol Connections"
- [x] Clarify connection to router
- [x] Update examples
- [x] Add note about bidirectional nature

---

### 3.5 Update Other Docs ✅

**Files:** `docs/index.md`, `docs/protocols/README.md`

**Tasks:**
- [x] Update terminology throughout
- [x] Update architecture diagrams

---

## Phase 4: Additional Features (Week 4+)

### 4.1 Add Transport Settings to Router

**Files:**
- `apps/bridge/src/index.html`
- `apps/bridge/src/app.js`
- `apps/bridge/electron/main.js`

**Tasks:**
- [ ] Add transport selection to router creation/editing modal
- [ ] Checkboxes: WebSocket, QUIC, TCP
- [ ] Show active transports in router list
- [ ] Update router config to include transports

**Estimated Time:** 1-2 days

---

### 4.2 Improve Server Scanning

**Files:**
- `apps/bridge/src/app.js`
- `apps/bridge/src/index.html`

**Tasks:**
- [ ] Click discovered server to add it
- [ ] Rename option
- [ ] Option to create bridge to discovered server
- [ ] Better visual feedback during scan
- [ ] Show server capabilities/metadata

**Estimated Time:** 2-3 days

---

### 4.3 Clarify Outputs

**Files:**
- `apps/bridge/src/index.html`
- `apps/bridge/src/app.js`

**Tasks:**
- [ ] Rename "OUTPUT TARGETS" → "SAVED DESTINATIONS"
- [ ] Add help text: "Save destinations for use in mappings and test signals"
- [ ] Show they're not active connections
- [ ] Update all references

**Estimated Time:** 0.5 days

---

## Testing Plan

### Unit Tests
- [ ] Test protocol connection creation
- [ ] Test router selection
- [ ] Test connection status
- [ ] Test bidirectional communication
- [ ] Test direct bridges

### Integration Tests
- [ ] Test with multiple routers
- [ ] Test with multiple connections per protocol
- [ ] Test signal forwarding to router
- [ ] Test error handling (no router)

### User Testing
- [ ] Test UI with non-technical users
- [ ] Get feedback on terminology
- [ ] Test workflow understanding
- [ ] Iterate based on feedback

---

## Success Criteria

1. ✅ Users understand "ADD PROTOCOL" creates protocol connections
2. ✅ Users can see which router each connection uses
3. ✅ Signals actually forward to CLASP router (not just UI)
4. ✅ UI clearly shows protocol connections vs direct bridges
5. ✅ Documentation matches new model
6. ✅ Non-technical users can understand the flow

---

## Risk Mitigation

### Risk: Breaking Existing Configs

**Mitigation:**
- Keep backward compatibility during transition
- Migrate existing configs automatically
- Show migration message on first launch

### Risk: User Confusion During Transition

**Mitigation:**
- Show both old and new UI during transition
- Add help tooltips
- Provide migration guide

### Risk: Documentation Out of Sync

**Mitigation:**
- Update all docs in Phase 3
- Review all files before release
- Test with fresh users

---

## Timeline Estimate

- **Phase 1 (Critical Fixes):** 1 week
- **Phase 2 (UI Reorganization):** 1 week
- **Phase 3 (Documentation):** 1 week
- **Phase 4 (Additional Features):** 1-2 weeks
- **Testing & Iteration:** 1 week

**Total:** 4-6 weeks

---

*See `.internal/MASTER-CONSOLIDATION-PLAN.md` for detailed file-by-file updates.*
