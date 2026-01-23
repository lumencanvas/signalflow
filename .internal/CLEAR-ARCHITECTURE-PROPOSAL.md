# Clear Architecture Proposal

## Current Reality (What Actually Happens)

### 1. CLASP Server
- **Actually starts a CLASP router** (spawns `clasp-router` process)
- Standalone server that listens on port
- Other CLASP clients can connect
- ✅ **Functions as a real server**

### 2. Protocol Servers (OSC, MIDI, MQTT, etc.)
- **Starts a protocol server** (listens for that protocol)
- **ALSO creates a bridge** to CLASP (via `clasp-service`)
- Bridge connects to `target_addr: 'internal'`
- **What is "internal"?** Need to investigate - likely means "connect to first CLASP server" or "use default router"
- ✅ **Functions as a real server** for that protocol
- ❌ **Bridge connection is hidden** from user

### 3. Bridges
- Explicit source → target connections
- User configures both ends
- Automatically forwards signals
- ✅ **Clear purpose**

### 4. Mappings
- Signal routing rules with transforms
- Match source signals → apply transform → route to target
- ✅ **Clear purpose**

### 5. Outputs
- **Just saved destination configurations**
- Used in mappings and test signals
- Not active connections
- ❓ **Purpose unclear to users**

## The Problems

### Problem 1: "Internal" Router is Unclear
- Protocol servers connect to `target_addr: 'internal'`
- What does "internal" mean?
- Does it auto-connect to first CLASP server?
- Does it fail if no CLASP server exists?
- **This needs to be explicit in UI**

### Problem 2: Servers Create Hidden Bridges
- User adds "OSC Server"
- It functions as OSC server ✅
- It also creates bridge to CLASP ✅
- But bridge is invisible ❌
- User doesn't see the connection

### Problem 3: Terminology Confusion
- "ADD SERVER" suggests standalone server
- But protocol servers also create bridges
- User doesn't understand the dual nature

### Problem 4: Outputs Are Unclear
- What are they for?
- When do you need them?
- How do they relate to servers/bridges?

## Proposed Clear Architecture

### Core Principle: Make Everything Explicit

**Servers = Real Servers**
- CLASP Server = CLASP Router (real server)
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

## Implementation Plan

### Phase 1: Make "Internal" Explicit

1. **When adding protocol server:**
   - Show dropdown: "Connect to CLASP Router"
   - List all CLASP servers
   - Default: First CLASP server (or "None" if none exist)
   - Store which router it connects to

2. **Update bridge creation:**
   - Instead of `target_addr: 'internal'`
   - Use actual CLASP server address
   - Or show error if no CLASP server exists

3. **Show connection in UI:**
   - Each protocol server shows: "→ CLASP Router @ localhost:7330"
   - Or "Not connected to CLASP Router"

### Phase 2: Show Auto-Created Bridges

1. **When protocol server creates bridge:**
   - Also add to `state.bridges` array
   - Mark as `autoCreated: true`
   - Link to server: `serverId: '...'`

2. **In Bridges tab:**
   - Show auto-created bridges in separate section
   - Allow editing (change target router)
   - Allow deletion (disconnects from CLASP)

3. **Visual connection:**
   - Show: "OSC Server → Bridge → CLASP Router"
   - Make it clear they're connected

### Phase 3: Clarify Terminology

1. **Rename:**
   - "CLASP Server" → "CLASP Router"
   - "ADD SERVER" → "ADD SERVER" (keep, but clarify)
   - Add help text: "Starts a [protocol] server. Optionally connects to CLASP router."

2. **Modal updates:**
   - "ADD SERVER" modal
   - For protocol servers, add section: "CLASP Router Connection"
   - Checkbox: "Connect to CLASP Router"
   - Dropdown: Select which router
   - Help: "When enabled, creates a bridge that translates [protocol] messages to CLASP format."

### Phase 4: Clarify Outputs

1. **Rename:**
   - "OUTPUT TARGETS" → "SAVED DESTINATIONS"
   - Help text: "Save destination configurations for use in mappings and test signals. These are not active connections."

2. **Or remove:**
   - Use servers/bridges directly in mappings
   - Simpler model

## Recommended Path: Option B (Clarify Current)

### Why This Works

1. **Minimal code changes** - Just UI updates
2. **Maintains functionality** - Everything still works
3. **Makes connections explicit** - User sees what connects to what
4. **Flexible** - User can choose to connect or not

### Key Changes

1. **Make "internal" explicit:**
   - Protocol servers show which CLASP router they connect to
   - User can change it
   - User can disconnect (remove bridge)

2. **Show all bridges:**
   - Auto-created bridges appear in Bridges tab
   - User can see/edit/delete them
   - Clear labeling: "Auto" vs "User"

3. **Visual connections:**
   - Flow diagram shows all connections
   - Servers → Bridges → Routers
   - Clear visual flow

4. **Clarify Outputs:**
   - Rename to "Saved Destinations"
   - Explain purpose
   - Or remove if not needed

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

## Benefits

1. **Explicit connections** - User sees what connects to what
2. **Flexible** - Can connect or not connect to CLASP
3. **Clear terminology** - Servers are servers, bridges are bridges
4. **Visual clarity** - Flow diagram shows everything
5. **Easy to understand** - Digital artists can follow the flow

## Implementation Checklist

- [ ] Investigate what "internal" means in bridge service
- [ ] Add CLASP router selection to protocol server modal
- [ ] Store which router each protocol server connects to
- [ ] Show connection status in server list
- [ ] Add auto-created bridges to `state.bridges`
- [ ] Show auto-created bridges in Bridges tab
- [ ] Add visual connection indicators
- [ ] Update terminology (CLASP Server → CLASP Router)
- [ ] Clarify or remove Outputs
- [ ] Update help text and tooltips
- [ ] Test with non-technical users
