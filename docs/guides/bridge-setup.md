# Router and Protocol Connection Setup Guide

This guide explains how to set up CLASP routers and protocol connections.

## Understanding CLASP Architecture

CLASP uses a **router-based architecture**:

```
┌─────────────────────────────────────────────────────────┐
│                    CLASP Router                         │
│              (Central message hub & state)              │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │  OSC     │  │  MIDI    │  │   DMX    │             │
│  │  Conn    │  │  Conn    │  │  Conn    │             │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘             │
└───────│─────────────│─────────────│─────────────────────┘
        │             │             │
        ▼             ▼             ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │  OSC    │   │  MIDI   │   │   DMX   │
   │ Devices │   │ Devices │   │Fixtures │
   └─────────┘   └─────────┘   └─────────┘
```

**Key Points:**
- A **CLASP Router** is the central message hub
- **Protocol Connections** translate between external protocols and CLASP
- Protocol connections connect TO a router (they are bidirectional)
- Multiple connections can use the same router

## Protocol Connections vs. Direct Bridges

**Important Distinction:**

### Protocol Connections (Default Behavior)
When you "add a protocol" in the desktop app or use CLI commands like `clasp osc`, you're creating a **protocol connection**:
- Listens for the external protocol (OSC, MIDI, etc.)
- Connects to a CLASP router automatically
- Translates messages bidirectionally between the external protocol and CLASP
- Routes everything through the CLASP router

**Example:** `clasp osc --port 9000` creates an OSC connection that listens on port 9000 and routes messages through the CLASP router.

### Direct Bridges (Protocol-to-Protocol)
Direct bridges connect two protocols without going through CLASP:
- OSC → MIDI direct translation
- Bypass CLASP router entirely
- For specialized use cases

**See also:** [Desktop App: Understanding Protocol Connections](./desktop-app-servers.md) for detailed explanation.

## Starting a CLASP Router

A router is required before you can add protocol connections. You have three options:

### Option 1: Desktop App (Easiest)

1. Download and install the CLASP Desktop App
2. The app automatically starts a router on `localhost:7330`
3. No configuration needed

### Option 2: CLI

```bash
# Start a router on default port 7331
clasp server

# Start on custom port
clasp server --port 7330

# Start with specific protocol
clasp server --protocol websocket --bind 0.0.0.0 --port 7330
```

### Option 3: Docker

```bash
docker run -p 7330:7330 lumencanvas/clasp-router
```

### Option 4: Embed in Your Application

See [Server Embedding Examples](../examples/README.md#rust-examples) for code examples.

## Setting Up Protocol Connections

**Important:** All protocol commands (`clasp osc`, `clasp mqtt`, etc.) create connections that route through the CLASP router. They are bidirectional - one connection handles both sending and receiving.

### OSC Connection

**What it does:** Translates OSC messages to/from CLASP

```bash
# Start OSC connection (listens for OSC, routes to CLASP router)
clasp osc --port 9000

# This creates a connection that:
# - Listens for OSC on UDP port 9000
# - Connects to CLASP router (default: localhost:7330)
# - Translates OSC ↔ CLASP bidirectionally
```

**Desktop App:**
1. Ensure CLASP router is running (or the app will start one automatically)
2. Click **ADD PROTOCOL** → Select **OSC (Open Sound Control)**
3. Configure bind address and port (default: 0.0.0.0:9000)
4. Click **START**

**What happens:**
- Listens for incoming OSC messages on the specified port
- Automatically connects to the CLASP router
- Translates OSC messages to CLASP format (and vice versa)
- Routes all messages through the CLASP router

**Example Flow:**
```
TouchOSC (OSC) → Port 9000 → OSC Connection → CLASP Router → Other Clients
```

### MIDI Connection

**What it does:** Translates MIDI messages to/from CLASP

```bash
# Start MIDI connection
clasp bridge --bridge-type midi --opt device="Launchpad X"

# List available MIDI devices
clasp bridge --bridge-type midi --opt list-devices=true
```

**Desktop App:**
1. Click **ADD PROTOCOL** → Select **MIDI (Musical Instruments)**
2. Select MIDI input/output devices
3. Click **START**

**What happens:**
- Creates a MIDI connection to selected MIDI devices
- Automatically connects to the CLASP router
- Translates MIDI messages (notes, CC, etc.) to CLASP format (and vice versa)
- Makes MIDI devices accessible through CLASP

**Example Flow:**
```
MIDI Controller → MIDI Connection → CLASP Router → Lighting Software
```

### DMX Connection

**What it does:** Translates DMX channels from CLASP (output only)

```bash
# Start DMX connection (requires USB DMX interface)
clasp bridge --bridge-type dmx --opt device="/dev/ttyUSB0"
```

**Desktop App:**
1. Click **ADD PROTOCOL** → Select **DMX (USB Serial Interface)**
2. Select USB DMX device
3. Configure universe (default: 0)
4. Click **START**

**What happens:**
- Creates a DMX connection to the USB DMX interface
- Automatically connects to the CLASP router
- Translates CLASP messages to DMX channel updates
- Makes DMX fixtures controllable via CLASP

**Example Flow:**
```
CLASP Router → DMX Connection → USB Interface → DMX Fixtures
```

### Art-Net Connection

**What it does:** Translates Art-Net (DMX over Ethernet) to/from CLASP

```bash
# Start Art-Net connection
clasp bridge --bridge-type artnet --opt bind="0.0.0.0:6454"
```

**Desktop App:**
1. Click **ADD PROTOCOL** → Select **Art-Net (DMX over Ethernet)**
2. Configure bind address and port (default: 0.0.0.0:6454)
3. Configure subnet and universe
4. Click **START**

**What happens:**
- Creates an Art-Net connection that listens for Art-Net packets
- Automatically connects to the CLASP router
- Translates Art-Net DMX data to/from CLASP format
- Makes Art-Net nodes accessible through CLASP

**Example Flow:**
```
CLASP Router → Art-Net Connection → Network → Art-Net Nodes → DMX Fixtures
```

### MQTT Connection

**What it does:** Connects to MQTT broker and translates topics to/from CLASP

```bash
# Connect to MQTT broker
clasp mqtt --host localhost --port 1883

# Subscribe to specific topics
clasp mqtt --host broker.example.com --topic "sensors/#" --topic "home/+"
```

**Desktop App:**
1. Click **ADD PROTOCOL** → Select **MQTT (IoT Messaging)**
2. Enter broker host and port
3. Configure topics (comma-separated)
4. Add authentication if needed
5. Click **START**

**What happens:**
- Creates an MQTT connection to the MQTT broker
- Automatically connects to the CLASP router
- Subscribes to specified topics and translates to CLASP
- Publishes CLASP messages to MQTT topics

**Example Flow:**
```
IoT Sensors → MQTT Broker → MQTT Connection → CLASP Router → Control Panel
```

### WebSocket Connection

**What it does:** Provides WebSocket server that speaks CLASP protocol

```bash
# Start WebSocket connection (CLASP protocol)
clasp websocket --mode server --url 0.0.0.0:8080
```

**Note:** This is different from a CLASP router's native WebSocket. This connection can translate between generic WebSocket JSON and CLASP.

**Desktop App:**
1. Click **ADD PROTOCOL** → Select **WebSocket (JSON Bridge)**
2. Configure bind address and port
3. Select message format (JSON or MsgPack)
4. Click **START**

**What happens:**
- Creates a WebSocket connection that accepts WebSocket connections
- Automatically connects to the CLASP router
- Translates between WebSocket JSON/MsgPack and CLASP format
- Makes generic WebSocket clients accessible through CLASP

**Note:** This is different from a CLASP router's native WebSocket (which uses binary CLASP protocol). This connection translates generic WebSocket JSON to CLASP.

### HTTP Connection

**What it does:** Provides REST API that translates HTTP requests to/from CLASP

```bash
# Start HTTP REST API
clasp http --bind 0.0.0.0:3000
```

**Desktop App:**
1. Click **ADD PROTOCOL** → Select **HTTP REST API**
2. Configure bind address and port
3. Set base path (optional, default: /api)
4. Click **START**

**What happens:**
- Creates an HTTP connection that provides REST API endpoints
- Automatically connects to the CLASP router
- Translates HTTP requests/responses to/from CLASP format
- Makes CLASP accessible via REST API

**Example Flow:**
```
Web Browser → HTTP GET /api/lights/brightness → HTTP Connection → CLASP Router
```

## Common Setup Patterns

### Pattern 1: Single Router, Multiple Connections

```
CLASP Router (localhost:7330)
├── OSC Connection (port 9000) → TouchOSC
├── MIDI Connection → Launchpad
├── DMX Connection → USB Interface
└── MQTT Connection → IoT Broker
```

**Setup:**
1. Start router: `clasp server` or Desktop App
2. Start each connection: `clasp osc --port 9000`, `clasp bridge --bridge-type midi`, etc.

### Pattern 2: Desktop App (All-in-One)

The Desktop App automatically runs:
- **CLASP Router** (internal, on localhost:7330)
- **All protocol connections** you configure via "ADD PROTOCOL"

**Important:** When you click "ADD PROTOCOL" in the desktop app:
- You're creating a **protocol connection**
- The connection automatically connects to the CLASP router
- The connection translates bidirectionally between the external protocol and CLASP
- No manual router setup needed - it's all automatic

### Pattern 3: Distributed (Multiple Routers)

```
Router A (studio.local:7330)
├── OSC Connection → Resolume
└── MIDI Connection → DAW

Router B (stage.local:7330)
├── Art-Net Connection → Lighting Network
└── DMX Connection → Backup USB

Routers connected via network
```

## Troubleshooting

### "Cannot connect to router"

**Problem:** Protocol connection can't find CLASP router

**Solutions:**
1. Ensure router is running: `clasp server` or Desktop App
2. Check router port (default: 7330 for WebSocket, 7331 for QUIC)
3. Specify router URL: `clasp osc --router ws://localhost:7330`

### "Port already in use"

**Problem:** Another application is using the port

**Solutions:**
1. Find what's using the port: `lsof -i :9000` (macOS/Linux) or `netstat -ano | findstr :9000` (Windows)
2. Use a different port: `clasp osc --port 9001`
3. Stop the conflicting application

### "MIDI device not found"

**Problem:** MIDI connection can't find device

**Solutions:**
1. List devices: `clasp bridge --bridge-type midi --opt list-devices=true`
2. Use exact device name: `clasp bridge --bridge-type midi --opt device="Exact Device Name"`
3. Check device permissions (macOS may need permission)

### "DMX interface not accessible"

**Problem:** Can't access USB DMX interface

**Solutions:**
1. Check device path: `ls /dev/tty*` (Linux) or Device Manager (Windows)
2. Check permissions: `sudo chmod 666 /dev/ttyUSB0` (Linux)
3. Ensure interface is not in use by another app

## Next Steps

- See [Protocol Mapping Examples](./protocol-mapping.md) for detailed message translation examples
- Check [Protocol-Specific Guides](../protocols/README.md) for advanced configuration
- Review [Integration Examples](../integrations/README.md) for real-world setups
