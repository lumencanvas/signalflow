# CLASP

**Creative Low-Latency Application Streaming Protocol**

CLASP is a universal protocol router for creative applications. It connects disparate protocols like OSC, MIDI, DMX, Art-Net, MQTT, WebSockets, Socket.IO, and HTTP/REST into a unified, routable message system.

## What is CLASP?

CLASP provides:

- **Protocol Connections** - Connect any protocol to the CLASP router for unified messaging
- **Signal Routing** - Route and transform signals between any connected protocols
- **REST API Gateway** - Stand up API endpoints that trigger protocol messages
- **Real-time Monitoring** - Watch signals flow through your system
- **Learn Mode** - Automatically capture addresses from incoming signals

## Use Cases

### Live Performance
Connect lighting (DMX/Art-Net), audio (OSC/MIDI), and video systems together. Map a MIDI controller to lighting cues or OSC messages.

### Installation Art
Bridge sensors and actuators across different protocols. Use MQTT for IoT devices, OSC for sound, and DMX for lighting.

### Home Automation
Create REST APIs that trigger home automation events. Map HTTP endpoints to MQTT topics or OSC messages.

### Software Integration
Connect creative tools that speak different protocols. Bridge TouchDesigner (OSC) with Ableton (MIDI) and custom WebSocket apps.

## Quick Start

```bash
# Install the CLI
cargo install clasp-cli

# Or download the desktop app from releases

# Start the CLASP router (central message hub)
clasp server --port 7330

# Add protocol connections (these connect to the router)
clasp osc --port 9000      # OSC on port 9000 → CLASP Router
clasp mqtt --host broker.local  # MQTT broker → CLASP Router
```

## Documentation

- [Getting Started](./getting-started/README.md)
- [Bridge Setup Guide](./guides/bridge-setup.md) - How to set up routers and protocol connections
- [Desktop App: Understanding Protocol Connections](./guides/desktop-app-servers.md) - How protocol connections work
- [Protocol Mapping Examples](./guides/protocol-mapping.md) - See how messages translate between protocols
- [Protocol Documentation](./protocols/) - Protocol-specific guides
- [Troubleshooting](./guides/troubleshooting.md)
- [Architecture](./architecture.md)

## Supported Protocols

| Protocol | Direction | Status |
|----------|-----------|--------|
| OSC | Bidirectional | ✅ Stable |
| MIDI | Bidirectional | ✅ Stable |
| DMX | Output | ✅ Stable |
| Art-Net | Bidirectional | ✅ Stable |
| MQTT | Bidirectional | ✅ Implemented |
| WebSocket | Bidirectional | ✅ Implemented |
| Socket.IO | Bidirectional | 🚧 Planned |
| HTTP/REST | Server + Client | ✅ Implemented |

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   OSC App   │     │  MQTT Hub   │     │  REST API   │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       ▼                   ▼                   ▼
┌──────────────────────────────────────────────────────┐
│                    CLASP Router                       │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│  │  OSC    │  │  MQTT   │  │  HTTP   │  │  MIDI   │  │
│  │  Conn   │  │  Conn   │  │  Conn   │  │  Conn   │  │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘  │
│                                                       │
│              Signal Routing & Transforms              │
└──────────────────────────────────────────────────────┘
       │                   │                   │
       ▼                   ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ MIDI Device  │   │   Lighting   │   │   Web App    │
└──────────────┘   └──────────────┘   └──────────────┘
```

## Desktop App

The CLASP desktop app provides a visual interface for:

- Creating and managing protocol connections
- Designing signal mappings with transforms
- Building REST API endpoints
- Monitoring real-time signal flow
- Learning addresses from incoming signals

![CLASP Desktop App](./assets/app-screenshot.png)

## License

MIT or Apache-2.0

---

*CLASP - Connect everything.*
