# Protocol Adapter Roles: Server vs Client

## The Answer: It Depends on the Protocol!

Protocol adapters can act as **servers**, **clients**, or **device interfaces** depending on the protocol. But they **always connect TO the CLASP router** (as a client).

---

## How Each Protocol Adapter Works

### 1. OSC Adapter
**Acts as:** **Server** (binds to UDP port)
- Listens on UDP port (e.g., `0.0.0.0:9000`)
- Receives OSC messages from external devices/apps
- Can also send OSC messages to remote addresses (client behavior)

**Connects to:** CLASP Router (as WebSocket client)

**Example:**
```
TouchOSC (client) → OSC Adapter (server on port 9000) → CLASP Router (client connection)
```

---

### 2. MQTT Adapter
**Acts as:** **Client** (connects to MQTT broker)
- Connects to MQTT broker (e.g., `localhost:1883`)
- Subscribes to topics
- Publishes to topics

**Connects to:** CLASP Router (as WebSocket client)

**Example:**
```
MQTT Adapter (client) → MQTT Broker (server) → IoT devices
MQTT Adapter (client) → CLASP Router (client connection)
```

---

### 3. WebSocket Adapter
**Acts as:** **Either Server OR Client** (user chooses)
- **Server mode:** Binds to TCP port, listens for connections
- **Client mode:** Connects to remote WebSocket server

**Connects to:** CLASP Router (as WebSocket client)

**Example (Server mode):**
```
Web App (client) → WebSocket Adapter (server on port 8080) → CLASP Router (client connection)
```

**Example (Client mode):**
```
WebSocket Adapter (client) → Remote WebSocket Server → CLASP Router (client connection)
```

---

### 4. HTTP Adapter
**Acts as:** **Either Server OR Client** (user chooses)
- **Server mode:** Binds to TCP port, listens for HTTP requests
- **Client mode:** Makes HTTP requests to remote servers

**Connects to:** CLASP Router (as WebSocket client)

**Example (Server mode):**
```
HTTP Client → HTTP Adapter (server on port 3000) → CLASP Router (client connection)
```

**Example (Client mode):**
```
HTTP Adapter (client) → Remote HTTP Server → CLASP Router (client connection)
```

---

### 5. MIDI Adapter
**Acts as:** **Device Interface** (opens MIDI ports)
- Opens MIDI input port (receives MIDI messages)
- Opens MIDI output port (sends MIDI messages)
- Not really a server or client - it's a device interface

**Connects to:** CLASP Router (as WebSocket client)

**Example:**
```
MIDI Controller → MIDI Adapter (device interface) → CLASP Router (client connection)
```

---

### 6. Art-Net Adapter
**Acts as:** **Server** (binds to UDP port)
- Listens on UDP port (e.g., `0.0.0.0:6454`)
- Receives Art-Net packets from lighting controllers
- Can also send Art-Net packets to remote addresses (client behavior)

**Connects to:** CLASP Router (as WebSocket client)

**Example:**
```
Lighting Controller → Art-Net Adapter (server on port 6454) → CLASP Router (client connection)
```

---

### 7. DMX Adapter
**Acts as:** **Device Interface** (opens serial port)
- Opens serial port (e.g., `/dev/ttyUSB0`)
- Sends DMX data (output only)
- Not really a server or client - it's a device interface

**Connects to:** CLASP Router (as WebSocket client)

**Example:**
```
DMX Adapter (device interface) → DMX Fixtures
DMX Adapter (device interface) → CLASP Router (client connection)
```

---

## Key Insight: Dual Role

Every protocol adapter has **two roles**:

1. **External Protocol Role:**
   - Server (OSC, HTTP server, WebSocket server, Art-Net)
   - Client (MQTT, HTTP client, WebSocket client)
   - Device Interface (MIDI, DMX)

2. **CLASP Router Role:**
   - **Always a client** - connects to CLASP router via WebSocket

---

## Visual Flow

```
┌─────────────────────────────────────────────────────────┐
│                    External World                       │
│                                                         │
│  TouchOSC  →  OSC Adapter (server)  →  CLASP Router   │
│  (client)      (listens on port)        (client conn)  │
│                                                         │
│  MQTT Broker ← MQTT Adapter (client) ←  CLASP Router   │
│  (server)      (connects to broker)     (client conn)  │
│                                                         │
│  MIDI Device → MIDI Adapter (device)  →  CLASP Router  │
│              (opens MIDI port)         (client conn)   │
└─────────────────────────────────────────────────────────┘
```

---

## UI Implications

### Current Confusion
- "ADD SERVER" suggests everything is a server
- But MQTT adapter is a client (connects to broker)
- But MIDI adapter is a device interface (opens ports)

### Better Terminology

**Option 1: "Protocol Adapter" (Generic)**
- "OSC Adapter" - works for server, client, or device
- "MQTT Adapter" - works for client
- "MIDI Adapter" - works for device interface

**Option 2: Be Specific**
- "OSC Server" (when it acts as server)
- "MQTT Client" (when it acts as client)
- "MIDI Interface" (when it's a device interface)

**Recommendation:** Use "Protocol Adapter" as the generic term, but show the role in the UI:
- "OSC Adapter (Server on port 9000)"
- "MQTT Adapter (Client to localhost:1883)"
- "MIDI Adapter (Device Interface)"

---

## Summary

**Question:** Does a protocol adapter act as a server or connect to a server?

**Answer:** 
- **For the external protocol:** It depends! Server (OSC, HTTP server), Client (MQTT, HTTP client), or Device Interface (MIDI, DMX)
- **For CLASP router:** Always a client - connects to CLASP router via WebSocket

**Key Point:** The adapter's role with the external protocol is separate from its connection to the CLASP router. It can be a server for external devices while being a client to the CLASP router.
