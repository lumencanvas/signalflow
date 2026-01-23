# Handoff Guide for Next LLM

**Date:** 2026-01-22  
**Context:** Protocol-Centric Architecture Model Implementation

---

## 🎯 START HERE

**Read these documents in order:**

1. **`.internal/COMPLETE-ARCHITECTURE-SUMMARY.md`** (5 min read)
   - **This is the definitive model** - everything is based on this
   - Explains the protocol-centric architecture
   - Shows what each component does
   - Terminology reference

2. **`.internal/MASTER-CONSOLIDATION-PLAN.md`** (15 min read)
   - **Comprehensive file-by-file update plan**
   - Shows exactly what needs to change in each file
   - Includes terminology matrix
   - User flow examples

3. **`.internal/IMPLEMENTATION-ROADMAP.md`** (10 min read)
   - Phase-by-phase implementation plan
   - Timeline estimates
   - Task breakdowns
   - Testing plan

---

## Quick Context

### What We Discovered

After deep analysis, we found:
1. **"Internal" router connection is NOT implemented** - Protocol connections create bridges but signals don't forward to CLASP router
2. **Bridges are hidden** - Auto-created bridges don't show in UI
3. **Terminology is misleading** - "ADD SERVER" suggests standalone servers, but they create bridges
4. **Protocol adapters are bidirectional** - One adapter handles both directions
5. **Transports are router settings** - Not separate components

### The Solution

**Protocol-Centric Organization:**
- Organize by Protocol (OSC, MIDI, MQTT, etc.) - matches user mental model
- Role (server/client/device) is a configuration setting, not primary organization
- Clear distinction: Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)

---

## What's Been Done

✅ **Complete architecture analysis**
- Mapped what each component actually does
- Identified all issues
- Determined best model (protocol-centric)

✅ **Comprehensive planning**
- File-by-file update plan
- Implementation roadmap
- Documentation update plan

✅ **Documentation created**
- Complete architecture summary
- Master consolidation plan
- Implementation roadmap
- Quick reference guide

---

## What Needs to Be Done

### Phase 1: Critical Fixes (START HERE)

**Priority:** CRITICAL - Nothing works without this

1. **Fix "internal" router connection**
   - File: `apps/bridge/electron/main.js`
   - Problem: Signals from protocol connections don't forward to CLASP router
   - Solution: Implement actual WebSocket connection to router and forward signals
   - See: `.internal/MASTER-CONSOLIDATION-PLAN.md` Section 1.1

2. **Separate routers from protocol connections**
   - File: `apps/bridge/src/app.js`
   - Problem: `state.servers` mixes routers and connections
   - Solution: Create `state.routers` array, separate logic
   - See: `.internal/MASTER-CONSOLIDATION-PLAN.md` Section 1.2

3. **Make router selection explicit**
   - Files: `apps/bridge/src/app.js`, `apps/bridge/src/index.html`
   - Problem: Uses magic string "internal"
   - Solution: Add dropdown to select router
   - See: `.internal/MASTER-CONSOLIDATION-PLAN.md` Section 1.3

### Phase 2: UI Reorganization

**Priority:** HIGH - Makes the model clear to users

1. Reorganize sidebar into sections
2. Update modal flow ("ADD PROTOCOL")
3. Update all terminology
4. Show connection status

### Phase 3: Documentation Updates

**Priority:** HIGH - Must match new model

1. Update README.md
2. Update all guides
3. Update CLI docs
4. Review protocol docs

---

## Key Files to Understand

### Critical Code Files

1. **`apps/bridge/electron/main.js`**
   - Backend Electron process
   - Handles `ipcMain.handle('start-server', ...)`
   - Creates bridges via `clasp-service`
   - **Needs:** Fix "internal" router connection

2. **`apps/bridge/src/app.js`**
   - Frontend state management
   - `state.servers` - currently mixes routers and connections
   - `state.bridges` - explicit bridges
   - `handleAddServer()` - creates protocol connections
   - **Needs:** Separate routers, update handlers

3. **`apps/bridge/src/index.html`**
   - UI structure
   - Modals, buttons, sidebar
   - **Needs:** Reorganize sections, update terminology

### Key Documentation Files

1. **`README.md`** - Main project readme
2. **`docs/guides/bridge-setup.md`** - Setup guide
3. **`docs/guides/desktop-app-servers.md`** - Needs complete rewrite
4. **`crates/clasp-cli/README.md`** - CLI documentation

---

## Terminology Reference

| Old | New | Use When |
|-----|-----|----------|
| "ADD SERVER" | "ADD PROTOCOL" | Button/modal |
| "CLASP Server" | "CLASP Router" | Router component |
| "OSC Server" | "OSC Connection" | When connected to CLASP |
| "Protocol Bridges" | "Protocol-to-Protocol Bridges" | Direct connections |
| "OUTPUT TARGETS" | "Saved Destinations" | Saved configs |

---

## Architecture Model

```
CLASP ROUTERS
  └── Router @ localhost:7330
      └── Transports: WebSocket, QUIC, TCP (settings)

PROTOCOL CONNECTIONS (organized by protocol)
  ├── OSC Connection (Server on 9000) → Router
  ├── MQTT Connection (Client to broker) → Router
  └── MIDI Connection (Device: Launchpad) → Router

DIRECT CONNECTIONS
  └── OSC → MIDI Bridge (bypasses CLASP)
```

**Key Points:**
- Protocol connections are bidirectional (one handles both directions)
- Role (server/client/device) is configuration, not organization
- All protocol connections connect to CLASP router
- Direct bridges bypass CLASP router

---

## Implementation Order

1. **Read:** `.internal/COMPLETE-ARCHITECTURE-SUMMARY.md`
2. **Read:** `.internal/MASTER-CONSOLIDATION-PLAN.md`
3. **Start:** Phase 1 - Fix "internal" router connection
4. **Then:** Phase 2 - UI reorganization
5. **Then:** Phase 3 - Documentation updates

---

## Questions to Answer First

Before starting implementation, make sure you understand:

1. ✅ What is a CLASP Router? (central message hub)
2. ✅ What is a Protocol Connection? (bidirectional adapter to CLASP)
3. ✅ What is a Direct Bridge? (protocol-to-protocol, bypasses CLASP)
4. ✅ Why protocol-centric? (matches user mental model)
5. ✅ What's the "internal" router problem? (signals don't forward)

If you can answer these, you're ready to start.

---

## Common Pitfalls to Avoid

1. **Don't organize by role** - Users think in protocols, not server/client
2. **Don't forget bidirectional** - One connection handles both directions
3. **Don't mix routers and connections** - Keep them separate
4. **Don't use "internal" magic string** - Make router selection explicit
5. **Don't forget to forward signals** - Bridge signals must go to router

---

## Success Criteria

You'll know you're done when:

1. ✅ Protocol connections actually forward signals to CLASP router
2. ✅ UI clearly shows protocol connections vs direct bridges
3. ✅ Users can select which router each connection uses
4. ✅ All documentation matches the new model
5. ✅ Non-technical users can understand the flow

---

## Related Documents

### Must Read
- `.internal/COMPLETE-ARCHITECTURE-SUMMARY.md` - **START HERE**
- `.internal/MASTER-CONSOLIDATION-PLAN.md` - **DETAILED PLAN**
- `.internal/IMPLEMENTATION-ROADMAP.md` - **PHASES**

### Reference
- `.internal/DEEP-UI-ARCHITECTURE-ANALYSIS.md` - UI/UX analysis
- `.internal/COMPONENT-CAPABILITIES-MAP.md` - Component capabilities
- `.internal/PROTOCOL-ADAPTER-ROLES.md` - Server vs Client roles
- `.internal/QUICK-REFERENCE.md` - Quick reference

### Master Plan
- `.internal/PLAN-HANDOFF.md` - Master plan with all tasks

---

## Next Steps

1. **Read the three "Must Read" documents** (30 minutes)
2. **Understand the model** - Protocol-centric organization
3. **Start Phase 1** - Fix "internal" router connection
4. **Follow the roadmap** - Phase 2, then Phase 3
5. **Test with users** - Get feedback, iterate

---

**Good luck! The model is solid, the plan is comprehensive, and everything is documented. You've got this! 🚀**
