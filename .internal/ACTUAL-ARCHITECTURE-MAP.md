# Desktop App: Actual Architecture Map

## What Each Component Actually Does

After tracing through the code, here's what everything actually does:

### 1. SERVERS (`state.servers`)

#### CLASP Server
**What it does:**
- **Actually starts a CLASP router** (spawns `clasp-router` process)
- Listens on specified port (default: localhost:7330)
- Accepts CLASP protocol connections
- **Standalone and functional** - other CLASP clients can connect
- Can enable mDNS discovery
- Can require authentication

**Code:** `startClaspServer()` spawns `clasp-router` binary

**User expectation:** ✅ Matches - it's a real server

#### Protocol Servers (OSC, MIDI, MQTT, WebSocket, HTTP, Art-Net, DMX, sACN)
**What it does:**
- **Starts a protocol server** (listens for that protocol)
- **ALSO creates a bridge** internally that connects to CLASP router
- Bridge connects to `target_addr: 'internal'` (internal CLASP router)
- The bridge is NOT added to `state.bridges` array
- So it doesn't show in "Protocol Bridges" tab

**Example - OSC Server:**
1. User adds "OSC Server" on port 9000
2. Backend calls `startOscServer(config)`
3. Sends `create_bridge` message:
   ```json
   {
     "type": "create_bridge",
     "source": "osc",
     "source_addr": "0.0.0.0:9000",
     "target": "clasp",
     "target_addr": "internal"  // ← Connects to internal router
   }
   ```
4. OSC server listens on port 9000 ✅
5. Bridge translates OSC → CLASP ✅
6. But bridge is hidden from user ❌

**User expectation:** ✅ Functions as OSC server, but ❌ user doesn't know it's also a bridge

**The Problem:** 
- User thinks: "I started an OSC server"
- Reality: "I started an OSC server AND created a bridge to CLASP"
- But the bridge part is invisible

### 2. BRIDGES (`state.bridges`)

**What they do:**
- Explicit source → target protocol connections
- User configures both source and target
- Automatically forward signals from source to target
- Shown in "Protocol Bridges" tab

**Example:**
- Source: OSC 0.0.0.0:9000
- Target: CLASP localhost:7330
- Result: All OSC messages forwarded to CLASP

**User expectation:** ✅ Matches - it's a bridge between protocols

### 3. MAPPINGS (`state.mappings`)

**What they do:**
- Signal routing rules with transforms
- Match incoming signals based on:
  - Protocol
  - Address pattern (with wildcards)
  - MIDI channel/note/CC
  - DMX universe/channel
- Apply transform (scale, invert, clamp, etc.)
- Route to target protocol/address

**Example:**
- Source: OSC `/fader1` (0.0-1.0)
- Transform: Scale 0-1 → 0-127
- Target: MIDI CC 7 on channel 1
- Result: OSC fader controls MIDI volume

**User expectation:** ✅ Matches - it's signal routing with transforms

### 4. OUTPUTS (`state.outputs`)

**What they do:**
- **Saved destination configurations**
- Used as targets in mappings
- Used in test signal generator
- Just configuration - doesn't create connections
- Examples: "Resolume @ 192.168.1.100:7000", "MIDI Device: Launchpad"

**User expectation:** ❓ Unclear - seems redundant with bridges/servers

**Reality:** They're just saved destinations, not active connections

## Current Architecture Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Desktop App                              │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   SERVERS    │  │   BRIDGES    │  │  MAPPINGS    │     │
│  │              │  │              │  │              │     │
│  │ CLASP Server │  │ OSC → CLASP  │  │ OSC /fader1  │     │
│  │ (spawns      │  │ MIDI → OSC   │  │ → MIDI CC 7  │     │
│  │  router)     │  │              │  │              │     │
│  │              │  │              │  │              │     │
│  │ OSC Server   │  │              │  │              │     │
│  │ (listens +   │  │              │  │              │     │
│  │  bridges to  │  │              │  │              │     │
│  │  CLASP)      │  │              │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                             │
│  ┌──────────────┐                                          │
│  │   OUTPUTS   │  (Just saved destinations)                │
│  │             │                                           │
│  │ "Resolume"  │                                           │
│  │ "Launchpad" │                                           │
│  └──────────────┘                                          │
└─────────────────────────────────────────────────────────────┘
```

## The Confusion Points

### 1. "Internal" CLASP Router

When protocol servers create bridges with `target_addr: 'internal'`, what does "internal" mean?

**Need to check:** Is there an auto-started CLASP router? Or does it connect to a user-created CLASP server?

**Looking at code:**
- Protocol servers create bridges to `target_addr: 'internal'`
- But there's no code that auto-starts a CLASP router
- So "internal" must mean: "connect to the first CLASP server in the list" or "connect to a default router"

**This is unclear and needs investigation.**

### 2. Servers vs Bridges

**Current:**
- Servers: Create protocol servers + hidden bridges
- Bridges: Explicit protocol-to-protocol connections
- **Overlap:** Both create bridges, but managed separately

**User confusion:**
- "Why isn't my OSC server in the Bridges tab?"
- "What's the difference between adding an OSC server and creating an OSC→CLASP bridge?"

### 3. Outputs Purpose

**Current:**
- Outputs are just saved destinations
- Used in mappings and test signals
- Don't create active connections

**User confusion:**
- "What are outputs for?"
- "Do I need to create an output to send to Resolume?"
- "Why not just use a bridge?"

## What's Needed for Everything to Work

### Minimum Setup:

1. **CLASP Router** (required)
   - At least one CLASP server must be running
   - This is the central hub

2. **Protocol Servers** (optional, for receiving)
   - OSC Server → receives OSC, bridges to CLASP
   - MIDI Server → receives MIDI, bridges to CLASP
   - etc.

3. **Bridges** (optional, for explicit routing)
   - OSC → CLASP (if you want explicit control)
   - CLASP → Art-Net (for sending)

4. **Mappings** (optional, for signal routing)
   - Route specific addresses with transforms

5. **Outputs** (optional, convenience)
   - Saved destinations for mappings/tests

### The Missing Piece: "Internal" Router

**What is "internal" CLASP router?**

After examining the code:
- `target_addr: 'internal'` is just a **placeholder string** stored in bridge config
- The bridge service (`clasp-service`) doesn't actually handle "internal" specially
- Bridges created by `clasp-service` only handle the **source protocol** (e.g., OSC)
- They emit `BridgeEvent::ToClasp(msg)` events that go to Electron via stdout
- **But there's no automatic forwarding to CLASP routers!**

**Current Flow:**
1. User adds "OSC Server"
2. Electron sends `create_bridge` with `target_addr: 'internal'`
3. Bridge service creates OSC bridge (listens for OSC on port)
4. OSC bridge emits `BridgeEvent::ToClasp(msg)` events
5. Electron receives events via `clasp-service` stdout
6. **❓ What happens next?** Events might be added to signal monitor, but are they forwarded to CLASP router?

**The Problem:**
- "Internal" suggests automatic connection to CLASP router
- But there's no code that actually connects bridge events to a CLASP router
- Events might just be displayed in the signal monitor, not forwarded

**This needs investigation and likely implementation.**

## Recommended Architecture Clarification

### Option A: Make Servers Actually Standalone

**Servers:**
- CLASP Server = Real CLASP router (standalone)
- Protocol Servers = Real protocol servers (standalone, no auto-bridge)
- User explicitly creates bridges if they want translation

**Bridges:**
- Explicit connections between servers/protocols
- User configures source and target

**Mappings:**
- Signal routing rules with transforms

**Outputs:**
- Remove or clarify as "saved destinations"

### Option B: Clarify Current Architecture

**Servers:**
- CLASP Server = Real CLASP router
- Protocol Servers = Protocol server + auto-bridge to CLASP
- Show the bridge connection in UI
- Make "internal" router explicit

**Bridges:**
- Explicit protocol-to-protocol (not auto-managed)
- Show all bridges (including auto-created ones from servers)

**Mappings:**
- Signal routing rules

**Outputs:**
- Rename to "Saved Destinations" or "Targets"
- Clarify they're just config, not active connections

### Option C: Unified Model

**Everything is a "Connection":**
- CLASP Router (server)
- Protocol Server (server + optional bridge)
- Bridge (explicit connection)
- All shown in one unified view

**Mappings:**
- Signal routing rules

**Outputs:**
- Remove or integrate into connections

## Recommended Path Forward

**Option B (Clarify Current)** is best because:
1. Minimal code changes
2. Maintains current functionality
3. Just needs UI clarity

**Specific Changes:**

1. **Make "internal" router explicit:**
   - Show which CLASP server protocol servers connect to
   - Auto-select first CLASP server, allow user to change
   - Show connection status

2. **Show auto-created bridges:**
   - When user adds "OSC Server", also show bridge in Bridges tab
   - Label it as "Auto-created from OSC Server"
   - Allow user to see/edit the bridge connection

3. **Clarify terminology:**
   - "CLASP Server" = "CLASP Router" (it's a router)
   - "OSC Server" = "OSC Server + Bridge to CLASP"
   - Show the bridge connection visually

4. **Clarify Outputs:**
   - Rename to "Saved Destinations"
   - Explain: "Save destinations for use in mappings and test signals"
   - Show they're not active connections

5. **Visual flow:**
   - Show: OSC Server → Bridge → CLASP Router
   - Make connections visible
   - Show what connects to what

## Implementation Priority

1. **HIGH:** Clarify what "internal" router means
2. **HIGH:** Show auto-created bridges in Bridges tab
3. **MEDIUM:** Visual connection indicators
4. **MEDIUM:** Clarify Outputs purpose
5. **LOW:** Terminology updates
