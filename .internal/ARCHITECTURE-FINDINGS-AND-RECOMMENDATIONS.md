# Architecture Findings & Recommendations

## Executive Summary

After tracing through the codebase, I've mapped out what each component actually does and identified several critical issues that make the UI/UX misleading. This document provides the "real truths" and recommends a clear path forward.

---

## What Actually Happens (Current Reality)

### 1. CLASP Server ✅
- **Actually starts a CLASP router** (spawns `clasp-router` process)
- Standalone server that listens on port (default: localhost:7330)
- Other CLASP clients can connect
- **Functions as a real server** ✅

### 2. Protocol Servers (OSC, MIDI, MQTT, etc.) ⚠️
- **Starts a protocol server** (listens for that protocol) ✅
- **Creates a bridge** via `clasp-service` that emits signals
- Bridge config has `target_addr: 'internal'` (just a placeholder string)
- **Signals go to frontend UI** (signal monitor) ✅
- **BUT: Signals are NOT automatically forwarded to CLASP router** ❌
- Bridge is NOT added to `state.bridges` array (hidden from user) ❌

**The Problem:**
- User thinks: "I started an OSC server that connects to CLASP"
- Reality: "I started an OSC server that displays signals in the UI, but doesn't actually connect to CLASP automatically"
- The "internal" router connection is **not implemented**

### 3. Bridges ✅
- Explicit source → target protocol connections
- User configures both ends
- Shown in "Protocol Bridges" tab
- **Clear purpose** ✅

### 4. Mappings ✅
- Signal routing rules with transforms
- Match source signals → apply transform → route to target
- **Clear purpose** ✅

### 5. Outputs ❓
- **Just saved destination configurations**
- Used in mappings and test signals
- Not active connections
- **Purpose unclear to users** ❓

---

## Critical Issues

### Issue 1: "Internal" Router Doesn't Exist

**Current Code:**
- Protocol servers create bridges with `target_addr: 'internal'`
- `clasp-service` stores this as a string (no special handling)
- Bridge emits signals that go to Electron frontend
- **Signals are NOT forwarded to any CLASP router**

**What Should Happen:**
- When `target_addr: 'internal'`, bridge should connect to a CLASP router
- Signals should be automatically forwarded to that router
- User should see which router it connects to

**What Actually Happens:**
- Signals just appear in the signal monitor
- User must create mappings or explicit bridges to route to CLASP
- "Internal" is misleading

### Issue 2: Hidden Bridges

**Current Code:**
- Protocol servers create bridges internally
- Bridges are NOT added to `state.bridges` array
- User doesn't see them in "Protocol Bridges" tab
- User doesn't know bridges exist

**The Problem:**
- User adds "OSC Server"
- Bridge is created but invisible
- User can't see/edit/delete the bridge
- Confusing when user also creates explicit bridges

### Issue 3: Terminology Confusion

**Current:**
- "ADD SERVER" suggests standalone server
- But protocol servers also create bridges (hidden)
- User doesn't understand dual nature

**User Expectation:**
- "ADD SERVER" = standalone server that functions independently
- "CREATE BRIDGE" = connection between protocols
- Clear separation

**Reality:**
- "ADD SERVER" = server + hidden bridge
- "CREATE BRIDGE" = explicit bridge
- Overlapping concepts

### Issue 4: Outputs Are Unclear

**Current:**
- "OUTPUT TARGETS" section
- Saved destination configurations
- Used in mappings and test signals

**User Confusion:**
- What are outputs for?
- When do I need them?
- How do they relate to servers/bridges?
- Are they active connections?

---

## Recommended Architecture (Clear & Flexible)

### Core Principle: Make Everything Explicit

**Servers = Real Servers**
- CLASP Router = CLASP Router (real server)
- OSC Server = OSC Server (real server, listens for OSC)
- MIDI Server = MIDI Server (real server, listens for MIDI)
- etc.

**Bridges = Explicit Connections**
- Show ALL bridges (including auto-created ones)
- Make connections visible
- User can see what connects to what

**Mappings = Signal Routing**
- Route specific signals with transforms
- Clear purpose

**Outputs = Saved Destinations (Optional)**
- Rename to "Saved Destinations" or remove
- Just convenience for mappings

### Proposed UI Structure

#### Sidebar: "SERVERS"

**CLASP Router**
- Label: "CLASP Router" (not "CLASP Server")
- Shows: Port, status, connections
- Action: "START ROUTER"

**Protocol Servers**
- Label: "OSC Server" (functions as OSC server)
- Shows: Port, status
- **NEW:** Shows "Bridge to: [CLASP Router name]" or "Not connected"
- Action: "START SERVER"
- **NEW:** Option to "Connect to CLASP Router" (checkbox or dropdown)

**Visual:**
```
┌─────────────────────────────┐
│ CLASP Router                │
│ localhost:7330  [Running]   │
│ 3 connections               │
└─────────────────────────────┘

┌─────────────────────────────┐
│ OSC Server                  │
│ 0.0.0.0:9000  [Running]     │
│ → Bridge to: CLASP Router   │ ← NEW
└─────────────────────────────┘
```

#### Bridges Tab: Show ALL Bridges

**Auto-Created Bridges:**
- Show bridges created by protocol servers
- Label: "Auto: OSC Server → CLASP Router"
- Allow user to edit/delete
- Show connection status

**User-Created Bridges:**
- Explicit bridges user created
- Label: "OSC → MIDI" or "CLASP → Art-Net"
- Full control

**Visual:**
```
┌─────────────────────────────────────┐
│ Auto-Created Bridges                │
│ ┌─────────────────────────────────┐ │
│ │ OSC Server → CLASP Router       │ │
│ │ [Auto] [Edit] [Delete]          │ │
│ └─────────────────────────────────┘ │
│                                      │
│ User-Created Bridges                │
│ ┌─────────────────────────────────┐ │
│ │ OSC → MIDI                       │ │
│ │ [Edit] [Delete]                  │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

#### Mappings Tab: Keep As Is
- Signal routing with transforms
- Clear purpose

#### Outputs: Clarify or Remove

**Option A: Remove**
- Just use bridges/servers directly in mappings

**Option B: Rename & Clarify**
- Rename to "Saved Destinations"
- Explain: "Save destinations for quick selection in mappings"
- Show they're not active connections

---

## Implementation Plan

### Phase 1: Fix "Internal" Router Connection

**Problem:** Signals from protocol servers don't automatically forward to CLASP routers

**Solution:**
1. When protocol server creates bridge with `target_addr: 'internal'`:
   - Find first running CLASP router (or show error if none)
   - Create WebSocket connection to that router
   - Forward all bridge signals to router
   - Store connection in bridge config

2. Update bridge creation:
   - Instead of `target_addr: 'internal'`
   - Use actual CLASP router address
   - Or show error if no CLASP router exists

3. Show connection in UI:
   - Each protocol server shows: "→ CLASP Router @ localhost:7330"
   - Or "Not connected to CLASP Router"

**Code Changes:**
- `apps/bridge/electron/main.js`: Add CLASP router connection logic
- Forward bridge signals to CLASP router via WebSocket
- Store router connection per bridge

### Phase 2: Show Auto-Created Bridges

**Problem:** Bridges created by protocol servers are hidden

**Solution:**
1. When protocol server creates bridge:
   - Also add to `state.bridges` array
   - Mark as `autoCreated: true`
   - Link to server: `serverId: '...'`

2. In Bridges tab:
   - Show auto-created bridges in separate section
   - Allow editing (change target router)
   - Allow deletion (disconnects from CLASP)

3. Visual connection:
   - Show: "OSC Server → Bridge → CLASP Router"
   - Make it clear they're connected

**Code Changes:**
- `apps/bridge/src/app.js`: Add auto-created bridges to `state.bridges`
- `apps/bridge/src/app.js`: Update `renderBridges()` to show auto-created section
- `apps/bridge/electron/main.js`: Return bridge info when creating protocol server

### Phase 3: Clarify Terminology

**Problem:** Terminology is confusing

**Solution:**
1. Rename:
   - "CLASP Server" → "CLASP Router"
   - "ADD SERVER" → "ADD SERVER" (keep, but clarify)
   - Add help text: "Starts a [protocol] server. Optionally connects to CLASP router."

2. Modal updates:
   - "ADD SERVER" modal
   - For protocol servers, add section: "CLASP Router Connection"
   - Checkbox: "Connect to CLASP Router"
   - Dropdown: Select which router
   - Help: "When enabled, creates a bridge that translates [protocol] messages to CLASP format."

**Code Changes:**
- `apps/bridge/src/index.html`: Update labels and add help text
- `apps/bridge/src/app.js`: Update server rendering and modals

### Phase 4: Clarify Outputs

**Problem:** Outputs purpose is unclear

**Solution:**
1. Rename:
   - "OUTPUT TARGETS" → "SAVED DESTINATIONS"
   - Help text: "Save destination configurations for use in mappings and test signals. These are not active connections."

2. Or remove:
   - Use servers/bridges directly in mappings
   - Simpler model

**Code Changes:**
- `apps/bridge/src/index.html`: Rename section
- `apps/bridge/src/app.js`: Update rendering and help text

---

## User Flow Examples

### Example 1: Simple Setup

**User wants:** TouchOSC to control lights via CLASP

**Steps:**
1. Add "CLASP Router" → Starts router on localhost:7330
2. Add "OSC Server" on port 9000
   - Checkbox: "Connect to CLASP Router" ✅
   - Dropdown: "CLASP Router @ localhost:7330"
3. Add "Art-Net Server"
   - Checkbox: "Connect to CLASP Router" ✅
   - Dropdown: "CLASP Router @ localhost:7330"
4. Create Mapping:
   - Source: OSC `/fader1`
   - Target: Art-Net Universe 1, Channel 1

**User sees:**
- Servers: CLASP Router, OSC Server, Art-Net Server
- Bridges tab: "OSC Server → CLASP Router" (auto), "Art-Net Server → CLASP Router" (auto)
- Mappings: OSC `/fader1` → Art-Net U1/C1

**User understands:** Everything is connected through CLASP router

### Example 2: Standalone OSC Server

**User wants:** Just an OSC server, no CLASP

**Steps:**
1. Add "OSC Server" on port 9000
   - Checkbox: "Connect to CLASP Router" ❌ (unchecked)

**User sees:**
- Servers: OSC Server (standalone)
- Bridges tab: Empty (no bridge created)
- OSC server functions normally, but no CLASP translation

**User understands:** It's a standalone OSC server

### Example 3: Multiple Routers

**User wants:** Two separate CLASP networks

**Steps:**
1. Add "CLASP Router A" on localhost:7330
2. Add "CLASP Router B" on localhost:7331
3. Add "OSC Server" on port 9000
   - Checkbox: "Connect to CLASP Router" ✅
   - Dropdown: "CLASP Router A @ localhost:7330"
4. Add "MIDI Server"
   - Checkbox: "Connect to CLASP Router" ✅
   - Dropdown: "CLASP Router B @ localhost:7331"

**User sees:**
- Servers: Router A, Router B, OSC Server, MIDI Server
- Bridges: "OSC Server → Router A", "MIDI Server → Router B"
- Clear separation of networks

**User understands:** Two separate CLASP networks

---

## Benefits

1. **Explicit connections** - User sees what connects to what
2. **Flexible** - Can connect or not connect to CLASP
3. **Clear terminology** - Servers are servers, bridges are bridges
4. **Visual clarity** - Flow diagram shows everything
5. **Easy to understand** - Digital artists can follow the flow

---

## Implementation Checklist

### Critical (Must Fix)
- [ ] **Fix "internal" router connection** - Actually forward signals to CLASP router
- [ ] **Show auto-created bridges** - Add to `state.bridges` and display in UI
- [ ] **Make router selection explicit** - Dropdown to select which CLASP router

### High Priority (Should Fix)
- [ ] **Update terminology** - CLASP Server → CLASP Router
- [ ] **Add connection indicators** - Show which router each server connects to
- [ ] **Clarify Outputs** - Rename or remove

### Medium Priority (Nice to Have)
- [ ] **Visual flow diagram** - Show all connections
- [ ] **Help tooltips** - Explain each concept
- [ ] **Test with users** - Get feedback from digital artists

---

## Files to Modify

### Backend (Electron)
- `apps/bridge/electron/main.js`
  - Add CLASP router connection logic
  - Forward bridge signals to CLASP router
  - Return bridge info when creating protocol servers

### Frontend
- `apps/bridge/src/app.js`
  - Add auto-created bridges to `state.bridges`
  - Update `renderBridges()` to show auto-created section
  - Update `renderServers()` to show connection status
  - Update server modals to include router selection

- `apps/bridge/src/index.html`
  - Update labels (CLASP Server → CLASP Router)
  - Add help text and tooltips
  - Update Outputs section

---

## Next Steps

1. **Review this document** - Confirm understanding
2. **Prioritize fixes** - Which issues are most critical?
3. **Implement Phase 1** - Fix "internal" router connection
4. **Test with users** - Get feedback on clarity
5. **Iterate** - Refine based on feedback
