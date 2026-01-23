# Desktop App Architecture: Servers vs Bridges

## Actual Implementation

After reviewing the code, here's what the desktop app actually does:

### Two Separate Concepts

1. **SERVERS** (`state.servers`) - Created via "ADD SERVER" button
   - Stored in sidebar "MY SERVERS" section
   - Includes: CLASP, OSC, MIDI, MQTT, WebSocket, HTTP, Art-Net, DMX, sACN, Socket.IO
   - When you add an "OSC Server", it:
     - Creates entry in `state.servers` array
     - Calls `startOscServer(config)` in backend
     - Backend sends `create_bridge` message to bridge service with:
       - `source: 'osc'`
       - `source_addr: '0.0.0.0:9000'`
       - `target: 'clasp'`
       - `target_addr: 'internal'` (connects to internal CLASP router)
   - **Does NOT appear in "Protocol Bridges" tab**

2. **BRIDGES** (`state.bridges`) - Created via "CREATE BRIDGE" button
   - Stored in "BRIDGES" tab
   - Explicit source → target connections
   - User specifies both source and target protocols/addresses
   - Example: OSC 0.0.0.0:9000 → CLASP localhost:7330
   - **Does NOT appear in "MY SERVERS" sidebar**

### Key Insight

**Servers ARE bridges internally**, but they're:
- Pre-configured to connect to internal CLASP router
- Managed as "servers" in the UI
- Automatically create bridges behind the scenes
- Not shown in the Bridges tab

**Bridges are**:
- User-configured source → target
- Shown in Bridges tab
- More flexible (can connect any protocol to any protocol)

## Current Behavior

### When User Clicks "ADD SERVER" → "OSC Server"

1. Modal shows: "ADD SERVER" with "Server Type: OSC"
2. User configures: bind address, port
3. User clicks: "START SERVER"
4. Backend creates: OSC bridge (OSC → CLASP internal router)
5. UI shows: "OSC Server @ 0.0.0.0:9000" in "MY SERVERS"
6. **Does NOT show in "Protocol Bridges" tab**

### When User Clicks "CREATE BRIDGE"

1. Modal shows: "CREATE BRIDGE" with Source and Target sections
2. User configures: Source protocol/address, Target protocol/address
3. User clicks: "CREATE"
4. Backend creates: Explicit bridge (source → target)
5. UI shows: "OSC 0.0.0.0:9000 → CLASP localhost:7330" in "Protocol Bridges"
6. **Does NOT show in "MY SERVERS" sidebar**

## The Confusion

**Problem:** The terminology is misleading:
- "ADD SERVER" suggests creating a standalone server
- But it actually creates a bridge to CLASP
- The bridge is hidden from the user
- User doesn't see it in "Protocol Bridges" tab

**Reality:**
- "ADD SERVER" = "Create protocol bridge that auto-connects to CLASP router"
- "CREATE BRIDGE" = "Create explicit protocol-to-protocol bridge"

## What Needs to Change

### Option 1: Keep Current Architecture, Clarify UI

**Servers Section:**
- Rename to "PROTOCOL CONNECTIONS" or "CONNECTED PROTOCOLS"
- Add subtitle: "Protocols connected to CLASP router"
- Show connection indicator: "→ CLASP Router"
- Tooltip: "Creates a bridge that connects [protocol] to CLASP"

**Bridges Tab:**
- Keep as is (explicit source → target bridges)
- Add note: "Servers in sidebar auto-create bridges to CLASP"

### Option 2: Unify Architecture

**Show servers as bridges:**
- When user adds "OSC Server", also show it in Bridges tab
- Show: "OSC 0.0.0.0:9000 → CLASP (internal)"
- Keep in both places (servers list + bridges tab)

**Or:**
- Remove separate "Bridges" tab
- Show all connections in one place
- Servers are just pre-configured bridges

### Option 3: Make Servers Actually Standalone

**Servers:**
- Create actual protocol servers (not bridges)
- Can run independently
- Optional: connect to CLASP router

**Bridges:**
- Explicit connections between servers/protocols

## Recommended Approach

**Option 1 (Clarify UI)** is best because:
- Minimal code changes
- Maintains current architecture
- Just needs better labeling and help text
- Users understand "connect protocol" better than "add server"

### Specific Changes Needed

1. **Sidebar "MY SERVERS":**
   - Rename to "CONNECTED PROTOCOLS" or "PROTOCOL CONNECTIONS"
   - Add help text: "Protocols connected to CLASP router. Messages are automatically translated."
   - Show each entry as: "OSC Bridge → CLASP Router" or badge "Bridge to CLASP"

2. **"ADD SERVER" Button:**
   - Rename to "CONNECT PROTOCOL" or "ADD PROTOCOL"
   - Tooltip: "Connect a protocol to CLASP. Creates a bridge automatically."

3. **Modal:**
   - Title: "CONNECT PROTOCOL" instead of "ADD SERVER"
   - Description: "Connect [protocol] devices to CLASP. A bridge will be created automatically that translates messages to CLASP format."
   - Button: "CONNECT" instead of "START SERVER"

4. **Server List Items:**
   - Show: "OSC Bridge" or "OSC → CLASP" instead of "OSC Server"
   - Add badge: "Bridge" or "→ CLASP Router"
   - Show connection status

5. **Bridges Tab:**
   - Add note: "Explicit protocol-to-protocol bridges. Servers in sidebar auto-create bridges to CLASP."

## Implementation Notes

- Servers are stored in `state.servers`
- Bridges are stored in `state.bridges`
- They're separate arrays, separate UI sections
- Servers internally create bridges via `create_bridge` message
- But the bridge object is not added to `state.bridges`
- This is why they don't show up in Bridges tab

## Code References

- Server creation: `apps/bridge/src/app.js:1545` (`handleAddServer`)
- Bridge creation: `apps/bridge/src/app.js:2106` (`handleCreateBridge`)
- Backend server start: `apps/bridge/electron/main.js:1077` (`start-server`)
- Backend bridge creation: `apps/bridge/electron/main.js:915` (`create-bridge`)
- OSC server creates bridge: `apps/bridge/electron/main.js:541` (`startOscServer`)
