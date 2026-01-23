# Component Capabilities Map

## What Each Component Actually Does & Can Do

### 1. CLASP Router

**What it is:**
- The central message hub for CLASP protocol
- Routes messages between CLASP clients
- Manages state, subscriptions, sessions

**Capabilities:**
- ✅ Accepts CLASP protocol connections
- ✅ Routes messages between clients
- ✅ Manages state (SET/PUBLISH)
- ✅ Handles subscriptions
- ✅ Can enable/disable **transports** (WebSocket, QUIC, TCP)
  - Transports are **settings on the router**, not separate components
  - Router can run multiple transports simultaneously
- ✅ Can require authentication
- ✅ Can announce via mDNS

**Current UI:** "ADD SERVER" → "CLASP Server"
**Better Name:** "CLASP Router"
**Location:** Separate section "CLASP Routers"

---

### 2. Protocol Adapters (Bridges)

**What they are:**
- **Bidirectional** translators between external protocols and CLASP
- Listen for external protocol messages → translate to CLASP
- Receive CLASP messages → translate to external protocol

**Capabilities:**
- ✅ **Bidirectional** (most protocols)
  - OSC: Bidirectional ✅
  - MIDI: Bidirectional ✅
  - MQTT: Bidirectional ✅
  - WebSocket: Bidirectional ✅
  - HTTP: Bidirectional ✅
  - Art-Net: Bidirectional ✅
  - sACN: Bidirectional (with mode: Sender/Receiver/Bidirectional) ✅
  - DMX: **Output only** ❌ (no input)
- ✅ Connect to CLASP router (via WebSocket) - **Always a client to the router**
- ✅ Translate protocol messages to/from CLASP format
- ✅ Use namespaces (e.g., `/osc`, `/midi`, `/mqtt`)

**External Protocol Role:**
- **OSC:** Acts as **server** (binds to UDP port)
- **MQTT:** Acts as **client** (connects to MQTT broker)
- **WebSocket:** Can be **server** (binds) or **client** (connects)
- **HTTP:** Can be **server** (binds) or **client** (makes requests)
- **MIDI:** Acts as **device interface** (opens MIDI ports)
- **Art-Net:** Acts as **server** (binds to UDP port)
- **DMX:** Acts as **device interface** (opens serial port)

**Key Insight:** The adapter's role with the external protocol (server/client/device) is separate from its connection to CLASP router (always a client).

**Current UI:** "ADD SERVER" → "OSC Server", "MIDI Server", etc.
**Better Name:** "Protocol Adapter" (show role: "OSC Adapter (Server on port 9000)")
**Location:** Separate section "Protocol Adapters"

**Key Insight:** Since they're bidirectional, you don't need separate "input" and "output" adapters. One adapter handles both directions.

---

### 3. Standalone Protocol Servers

**What they are:**
- Pure protocol servers (no CLASP translation)
- Just listen for that protocol, no CLASP connection
- For apps that only speak that protocol

**Capabilities:**
- ✅ Listen for protocol messages
- ❌ **No CLASP translation**
- ❌ **No connection to CLASP router**

**Current UI:** Doesn't exist (planned feature)
**Better Name:** "Standalone [Protocol] Server"
**Location:** Separate section "Standalone Servers"

**Use Case:** When you just need a protocol server, no CLASP involved.

---

### 4. Bridges (Explicit Connections)

**What they are:**
- Explicit source → target protocol connections
- User configures both ends
- Currently in "Protocol Bridges" tab

**Capabilities:**
- ✅ Forward signals from source to target
- ✅ Can be between any two protocols
- ✅ Shown in "Protocol Bridges" tab

**Question:** If protocol adapters are bidirectional, do we need explicit bridges?

**Answer:** Maybe not! If you have:
- OSC Adapter → CLASP Router
- MIDI Adapter → CLASP Router

Then OSC messages automatically flow: OSC → CLASP → MIDI (via router)

**But:** Explicit bridges might be useful for:
- Direct protocol-to-protocol (bypassing CLASP)
- Custom routing rules
- Legacy support

**Recommendation:** Keep bridges for advanced users, but make adapters the primary way.

---

### 5. Transports (Router Settings)

**What they are:**
- **Settings on CLASP Router**, not separate components
- How CLASP router accepts connections (WebSocket, QUIC, TCP)

**Capabilities:**
- ✅ WebSocket (default, works everywhere)
- ✅ QUIC (high-performance, requires UDP)
- ✅ TCP (simple fallback)
- ✅ Router can run multiple transports simultaneously

**Current UI:** Not exposed (hardcoded to WebSocket)
**Better Name:** "Transport Settings" (in router config)
**Location:** Settings/Config for CLASP Router

**Recommendation:** Add transport selection to router creation/editing.

---

### 6. Outputs (Saved Destinations)

**What they are:**
- **Just saved destination configurations**
- Not active connections
- Used in mappings and test signals

**Capabilities:**
- ✅ Save destination configs
- ✅ Use in mappings
- ✅ Use in test signals
- ❌ **Not active connections**

**Current UI:** "OUTPUT TARGETS"
**Better Name:** "Saved Destinations" or "Connection Targets"
**Location:** Keep as is, but clarify purpose

**Question:** If adapters are bidirectional, do we need outputs?

**Answer:** Maybe for:
- Quick selection in mappings
- Test signals
- But not for active connections (adapters handle that)

---

## Proposed UI Structure

### Sidebar Organization

```
┌─────────────────────────────────────┐
│  CLASP ROUTERS                      │
│  ┌───────────────────────────────┐ │
│  │ CLASP Router @ localhost:7330  │ │
│  │ [Running] [Edit] [Delete]     │ │
│  │ Transports: WebSocket ✅        │ │
│  └───────────────────────────────┘ │
│  [+ ADD ROUTER]                     │
├─────────────────────────────────────┤
│  PROTOCOL ADAPTERS                  │
│  ┌───────────────────────────────┐ │
│  │ OSC Adapter @ 0.0.0.0:9000    │ │
│  │ → Connected to: CLASP Router  │ │
│  │ [Running] [Edit] [Delete]     │ │
│  └───────────────────────────────┘ │
│  ┌───────────────────────────────┐ │
│  │ MIDI Adapter                   │ │
│  │ → Connected to: CLASP Router   │ │
│  │ [Running] [Edit] [Delete]      │ │
│  └───────────────────────────────┘ │
│  [+ ADD ADAPTER]                    │
├─────────────────────────────────────┤
│  STANDALONE SERVERS                 │
│  (No CLASP translation)             │
│  ┌───────────────────────────────┐ │
│  │ OSC Server @ 0.0.0.0:8000     │ │
│  │ (Standalone, no CLASP)         │ │
│  │ [Running] [Edit] [Delete]      │ │
│  └───────────────────────────────┘ │
│  [+ ADD STANDALONE SERVER]          │
└─────────────────────────────────────┘
```

### Tabs

**Bridges Tab:**
- Show explicit bridges (if needed)
- Maybe rename to "Advanced Bridges" or "Direct Connections"
- For power users who want protocol-to-protocol without CLASP

**Mappings Tab:**
- Signal routing with transforms
- Keep as is

**Outputs Tab:**
- Rename to "Saved Destinations" or "Connection Targets"
- Clarify: "Save destinations for use in mappings and test signals"
- Show they're not active connections

---

## Key Insights

### 1. Adapters Are Bidirectional

**Implication:** You don't need separate "input" and "output" adapters. One adapter handles both:
- External protocol → CLASP (via adapter → router)
- CLASP → External protocol (via router → adapter)

**UI Impact:** Simplify! Just "ADD ADAPTER" for each protocol.

### 2. Transports Are Router Settings

**Implication:** Transports aren't separate components. They're settings on the router.

**UI Impact:** Add transport selection to router creation/editing modal.

### 3. Adapters Connect to Routers

**Implication:** When you add an adapter, you select which router it connects to.

**UI Impact:** 
- Adapter modal: "Connect to: [Dropdown of routers]"
- Show connection status in adapter list

### 4. Standalone Servers Are Separate

**Implication:** Some users just want protocol servers, no CLASP.

**UI Impact:** Separate section for standalone servers.

### 5. Outputs Are Just Saved Configs

**Implication:** Outputs aren't active connections. They're just saved destinations.

**UI Impact:** Clarify purpose, maybe rename.

---

## Recommended Changes

### 1. Reorganize Sidebar

**Current:**
- "MY SERVERS" (everything mixed together)

**Proposed:**
- "CLASP ROUTERS" (section)
- "PROTOCOL ADAPTERS" (section)
- "STANDALONE SERVERS" (section)

### 2. Update Terminology

**Current → Proposed:**
- "CLASP Server" → "CLASP Router"
- "OSC Server" → "OSC Adapter" (when connected to CLASP)
- "OSC Server" → "OSC Server" (when standalone)
- "ADD SERVER" → "ADD ROUTER" / "ADD ADAPTER" / "ADD STANDALONE SERVER"

### 3. Add Router Transport Settings

**Current:** Hardcoded to WebSocket

**Proposed:** 
- Router creation modal: "Transports" section
- Checkboxes: WebSocket, QUIC, TCP
- Show active transports in router list

### 4. Make Adapter Connection Explicit

**Current:** `target_addr: 'internal'` (magic string)

**Proposed:**
- Adapter creation modal: "Connect to CLASP Router" dropdown
- Show which router each adapter connects to
- Error if no router exists

### 5. Clarify Outputs

**Current:** "OUTPUT TARGETS" (unclear)

**Proposed:**
- "SAVED DESTINATIONS" or "CONNECTION TARGETS"
- Help text: "Save destinations for use in mappings and test signals. These are not active connections."

---

## Implementation Priority

### High Priority
1. **Reorganize sidebar** into sections
2. **Update terminology** (Router, Adapter, Standalone Server)
3. **Make adapter connection explicit** (dropdown to select router)
4. **Add transport settings** to router

### Medium Priority
5. **Clarify outputs** purpose
6. **Show connection status** in adapter list
7. **Add standalone server** section

### Low Priority
8. **Simplify bridges** (maybe hide if adapters handle everything)
9. **Visual flow diagram** showing connections

---

## User Flow Examples

### Example 1: Simple Setup

**User wants:** TouchOSC to control lights via CLASP

**Steps:**
1. Add "CLASP Router" → Starts router on localhost:7330
2. Add "OSC Adapter" on port 9000
   - Dropdown: "Connect to: CLASP Router @ localhost:7330"
3. Add "Art-Net Adapter"
   - Dropdown: "Connect to: CLASP Router @ localhost:7330"
4. Create Mapping:
   - Source: OSC `/fader1`
   - Target: Art-Net Universe 1, Channel 1

**User sees:**
- Routers: CLASP Router @ localhost:7330
- Adapters: OSC Adapter → CLASP Router, Art-Net Adapter → CLASP Router
- Mappings: OSC `/fader1` → Art-Net U1/C1

**User understands:** Everything flows through CLASP router

### Example 2: Standalone OSC Server

**User wants:** Just an OSC server, no CLASP

**Steps:**
1. Add "Standalone OSC Server" on port 9000

**User sees:**
- Standalone Servers: OSC Server @ 0.0.0.0:9000 (no CLASP)

**User understands:** It's a pure OSC server, no CLASP translation

### Example 3: Multiple Routers

**User wants:** Two separate CLASP networks

**Steps:**
1. Add "CLASP Router A" on localhost:7330
2. Add "CLASP Router B" on localhost:7331
3. Add "OSC Adapter" on port 9000
   - Dropdown: "Connect to: CLASP Router A"
4. Add "MIDI Adapter"
   - Dropdown: "Connect to: CLASP Router B"

**User sees:**
- Routers: Router A, Router B
- Adapters: OSC → Router A, MIDI → Router B
- Clear separation

**User understands:** Two separate CLASP networks

---

## Benefits

1. **Clear separation** - Routers, Adapters, Standalone Servers are distinct
2. **Bidirectional adapters** - One adapter handles both directions
3. **Explicit connections** - User sees what connects to what
4. **Flexible** - Can use CLASP or just protocols
5. **Easy to understand** - Digital artists can follow the flow
