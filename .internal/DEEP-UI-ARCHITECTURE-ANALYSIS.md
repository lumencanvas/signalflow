# Deep UI/UX Architecture Analysis

## User Mental Models

### How Users Think

**Primary Mental Model: "I want to connect [protocol]"**
- "I want to connect my TouchOSC" → They think **OSC**, not "OSC server"
- "I want to connect to an MQTT broker" → They think **MQTT**, not "MQTT client"  
- "I want to connect my MIDI controller" → They think **MIDI**, not "MIDI device interface"
- "I want to connect my lights" → They think **Art-Net** or **DMX**, not "Art-Net server"

**Key Insight:** Users think in terms of **protocols**, not **roles** (server/client/device).

### Secondary Mental Model: "I want to connect [thing A] to [thing B]"
- "I want TouchOSC to control my lights" → OSC → Art-Net
- "I want my sensors to control everything" → MQTT → All protocols
- "I want my web app to control my setup" → WebSocket → All protocols

**Key Insight:** Users think in terms of **connections** and **flows**, not technical roles.

---

## Current Problems

### Problem 1: Role-Based Organization Doesn't Match Mental Model
- Organizing by "server/client/both" requires users to understand technical roles
- Digital artists don't think "I need an OSC server" - they think "I need OSC"
- MIDI isn't really server or client - it's a device interface

### Problem 2: "ADD SERVER" is Misleading
- Not everything is a server (MQTT is client, MIDI is device)
- Users don't understand what "server" means in this context
- Creates confusion about what they're actually creating

### Problem 3: Adapters vs Bridges Confusion
- Adapters connect protocols to CLASP (primary use case)
- Bridges connect protocols directly (advanced use case)
- But the distinction isn't clear in the UI

---

## Proposed Solution: Protocol-Centric Organization

### Core Principle: Organize by Protocol, Configure by Role

**Primary Organization:** By Protocol (matches user mental model)
**Secondary Configuration:** Role (server/client/device) is a setting

### UI Structure

```
┌─────────────────────────────────────────────────────────┐
│  SIDEBAR                                                 │
│                                                          │
│  ┌───────────────────────────────────────────────────┐ │
│  │  CLASP ROUTERS                                    │ │
│  │  ┌─────────────────────────────────────────────┐ │ │
│  │  │ CLASP Router @ localhost:7330                │ │ │
│  │  │ [Running] [Edit] [Delete]                    │ │ │
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
│  │  │ Server on 0.0.0.0:9000                       │ │ │
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
│  │  │ Direct connection (no CLASP)                 │ │ │
│  │  │ [Running] [Edit] [Delete]                     │ │ │
│  │  └─────────────────────────────────────────────┘ │ │
│  │                                                    │ │
│  │  [+ CREATE DIRECT BRIDGE]                         │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

---

## "ADD PROTOCOL" Modal Flow

### Step 1: Select Protocol
```
┌─────────────────────────────────────┐
│  ADD PROTOCOL CONNECTION            │
│                                     │
│  Select Protocol:                   │
│  ┌───────────────────────────────┐ │
│  │ [OSC]                         │ │
│  │ [MIDI]                        │ │
│  │ [MQTT]                        │ │
│  │ [WebSocket]                   │ │
│  │ [HTTP]                         │ │
│  │ [Art-Net]                      │ │
│  │ [DMX]                          │ │
│  │ [sACN]                         │ │
│  └───────────────────────────────┘ │
│                                     │
│  [Cancel]  [Next →]                 │
└─────────────────────────────────────┘
```

### Step 2: Configure Connection (Role is a Setting)

**For OSC (Server):**
```
┌─────────────────────────────────────┐
│  Configure OSC Connection           │
│                                     │
│  Connection Type:                   │
│  ○ Server (listen for OSC)          │
│  ● Client (connect to OSC server)   │
│                                     │
│  Server Settings:                   │
│  Bind Address: [0.0.0.0:9000]       │
│                                     │
│  Connect to CLASP Router:           │
│  [▼ CLASP Router @ localhost:7330] │
│                                     │
│  [← Back]  [Create Connection]      │
└─────────────────────────────────────┘
```

**For MQTT (Client):**
```
┌─────────────────────────────────────┐
│  Configure MQTT Connection          │
│                                     │
│  Connection Type:                   │
│  ● Client (connect to broker)       │
│                                     │
│  Broker Settings:                   │
│  Host: [localhost]                  │
│  Port: [1883]                       │
│  Topics: [#]                        │
│                                     │
│  Connect to CLASP Router:           │
│  [▼ CLASP Router @ localhost:7330] │
│                                     │
│  [← Back]  [Create Connection]      │
└─────────────────────────────────────┘
```

**For MIDI (Device):**
```
┌─────────────────────────────────────┐
│  Configure MIDI Connection          │
│                                     │
│  Connection Type:                   │
│  ● Device Interface                 │
│                                     │
│  Device Settings:                   │
│  Input Port:  [▼ Launchpad Pro]     │
│  Output Port: [▼ Launchpad Pro]     │
│                                     │
│  Connect to CLASP Router:           │
│  [▼ CLASP Router @ localhost:7330] │
│                                     │
│  [← Back]  [Create Connection]      │
└─────────────────────────────────────┘
```

**Key Insight:** Role (server/client/device) is a **configuration setting**, not the primary choice. The primary choice is the protocol.

---

## Sidebar Organization: By Protocol, Not Role

### Why NOT Organize by Role

**Problems with Role-Based Organization:**
1. **MIDI doesn't fit** - It's not server or client, it's a device interface
2. **User confusion** - Users don't think in terms of roles
3. **Bidirectional adapters** - Most adapters handle both directions anyway
4. **Redundant** - If adapters are bidirectional, why separate input/output?

**Example of Confusion:**
```
PROTOCOL SERVERS:
  - OSC Server
  - Art-Net Server

PROTOCOL CLIENTS:
  - MQTT Client
  - WebSocket Client

DEVICE INTERFACES:
  - MIDI Interface
  - DMX Interface

❓ Where does WebSocket server mode go?
❓ What if I want OSC to also send (client behavior)?
❓ Why are MIDI and DMX separate from servers/clients?
```

### Why Organize by Protocol

**Benefits:**
1. **Matches user mental model** - Users think "I need OSC", not "I need an OSC server"
2. **Clear grouping** - All OSC stuff together, all MIDI stuff together
3. **Flexible** - Can have multiple connections per protocol (e.g., OSC on port 9000 and 8000)
4. **Simple** - One section, not three

**Example:**
```
PROTOCOL CONNECTIONS:
  - OSC Connection (Server on 9000) → CLASP Router
  - OSC Connection (Server on 8000) → CLASP Router
  - MQTT Connection (Client to broker) → CLASP Router
  - MIDI Connection (Device: Launchpad) → CLASP Router
  - WebSocket Connection (Server on 8080) → CLASP Router
  - WebSocket Connection (Client to ws://...) → CLASP Router

✅ Clear: All protocols together
✅ Flexible: Multiple connections per protocol
✅ Simple: One section
```

---

## Direct Bridges: Protocol-to-Protocol

### When to Use Direct Bridges

**Use Direct Bridges When:**
- You want protocol-to-protocol connection **without CLASP**
- You want to bypass the CLASP router for performance
- You have legacy systems that need direct connections

**Example:**
- OSC → MIDI (direct, no CLASP translation)
- MQTT → HTTP (direct webhook)

### Naming: "Protocol-to-Protocol Bridge"

**Why This Name:**
- Makes it clear these are **direct connections**
- Distinguishes from protocol adapters (which go through CLASP)
- "Bridge" implies translation between protocols

**UI Location:**
- Separate section: "DIRECT CONNECTIONS" or "PROTOCOL-TO-PROTOCOL BRIDGES"
- Makes it clear these bypass CLASP

---

## Terminology Recommendations

### Primary Terms

| Current | Proposed | Rationale |
|---------|----------|-----------|
| "ADD SERVER" | "ADD PROTOCOL" | Matches user mental model (protocols, not roles) |
| "OSC Server" | "OSC Connection" | More accurate (it's a connection, not just a server) |
| "Protocol Bridges" | "Protocol-to-Protocol Bridges" | Clarifies these are direct connections |
| "OUTPUT TARGETS" | "Saved Destinations" | Clarifies purpose (saved configs, not active) |

### Secondary Terms (Show in UI, Not Primary)

- **Connection Type:** Server / Client / Device Interface
- **Role:** Shown as metadata, not primary organization
- **Status:** Connected to CLASP Router / Standalone

---

## User Flow Examples

### Example 1: TouchOSC to Lights

**User thinks:** "I want TouchOSC to control my lights"

**Steps:**
1. Add CLASP Router
2. Add Protocol → OSC
   - Connection Type: Server (default)
   - Bind: 0.0.0.0:9000
   - Connect to: CLASP Router
3. Add Protocol → Art-Net
   - Connection Type: Server (default)
   - Bind: 0.0.0.0:6454
   - Connect to: CLASP Router
4. Create Mapping: OSC `/fader1` → Art-Net U1/C1

**User sees:**
- Routers: CLASP Router
- Protocol Connections: OSC Connection, Art-Net Connection
- Both show "→ Connected to: CLASP Router"
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
- Protocol Connections: MQTT Connection (Client to localhost:1883)
- Shows "→ Connected to: CLASP Router"

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

## Implementation Recommendations

### 1. Primary Organization: By Protocol

**Sidebar Structure:**
```
CLASP ROUTERS
  - Router 1
  - Router 2
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

### 2. Modal: "ADD PROTOCOL"

**Flow:**
1. Select protocol (OSC, MIDI, MQTT, etc.)
2. Configure connection:
   - Connection type (server/client/device) - shown as setting, not primary choice
   - Protocol-specific settings
   - Connect to CLASP Router (dropdown)
3. Create connection

### 3. Show Connection Status

**In List:**
- "OSC Connection (Server on 9000) → Connected to: CLASP Router"
- "MQTT Connection (Client to localhost:1883) → Connected to: CLASP Router"
- "MIDI Connection (Device: Launchpad) → Connected to: CLASP Router"

### 4. Direct Bridges: Separate Section

**Naming:** "Protocol-to-Protocol Bridges" or "Direct Connections"
**Purpose:** Make it clear these bypass CLASP
**Location:** Separate section in sidebar

---

## Benefits of This Approach

1. **Matches User Mental Model** - Users think in protocols, not roles
2. **Clear Organization** - All OSC together, all MIDI together
3. **Flexible** - Multiple connections per protocol
4. **Simple** - One section, not three
5. **Clear Distinction** - Protocol Connections (to CLASP) vs Direct Bridges (bypass CLASP)
6. **Role is Configuration** - Server/client/device is a setting, not primary choice

---

## Comparison: Role-Based vs Protocol-Based

### Role-Based (User's Suggestion)
```
PROTOCOL SERVERS
  - OSC Server
  - Art-Net Server

PROTOCOL CLIENTS
  - MQTT Client
  - WebSocket Client

DEVICE INTERFACES
  - MIDI Interface
  - DMX Interface

❌ Problems:
- MIDI doesn't fit cleanly
- WebSocket can be server OR client
- Users don't think in roles
- Bidirectional adapters confuse the model
```

### Protocol-Based (Recommended)
```
PROTOCOL CONNECTIONS
  - OSC Connection (Server on 9000)
  - OSC Connection (Server on 8000)
  - MQTT Connection (Client to broker)
  - MIDI Connection (Device: Launchpad)
  - WebSocket Connection (Server on 8080)
  - WebSocket Connection (Client to ws://...)

✅ Benefits:
- Matches user mental model
- All protocols together
- Flexible (multiple per protocol)
- Role is configuration, not organization
```

---

## Final Recommendation

**Primary Organization:** By Protocol (not by role)
**Button:** "ADD PROTOCOL" (not "ADD SERVER")
**Modal:** Select protocol first, then configure role as a setting
**Sidebar:** "PROTOCOL CONNECTIONS" section (not "SERVERS/CLIENTS/DEVICES")
**Direct Bridges:** Separate section "DIRECT CONNECTIONS" or "PROTOCOL-TO-PROTOCOL BRIDGES"

**Rationale:**
- Matches how users think (protocols, not roles)
- Simpler mental model (one section, not three)
- More flexible (multiple connections per protocol)
- Role is a configuration detail, not primary organization
- Clear distinction between CLASP connections and direct bridges
