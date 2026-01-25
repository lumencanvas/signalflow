# CLASP Architecture

This document explains how CLASP is structured and when to use each component.

## Overview

CLASP is organized into layers, from low-level protocol to high-level applications:

```
┌─────────────────────────────────────────────────────────────┐
│                    APPLICATIONS                              │
│  Desktop App  │  CLI Tools  │  Your Custom App              │
├─────────────────────────────────────────────────────────────┤
│                     LIBRARIES                                │
│  clasp-client  │  clasp-router  │  clasp-bridge             │
├─────────────────────────────────────────────────────────────┤
│                    TRANSPORT                                 │
│  WebSocket  │  WebRTC  │  QUIC  │  UDP  │  BLE  │  Serial   │
├─────────────────────────────────────────────────────────────┤
│                      CORE                                    │
│  Types  │  Codec (binary)  │  Addresses  │  Security        │
├─────────────────────────────────────────────────────────────┤
│                    EMBEDDED                                  │
│  no_std client + server (bring your own transport)          │
└─────────────────────────────────────────────────────────────┘
```

## Crate Reference

### clasp-core

**Role:** Protocol foundation  
**Dependencies:** None  
**Size:** ~90KB compiled

Contains:
- Message types (SET, PUBLISH, SUBSCRIBE, etc.)
- Value types (Int, Float, Bool, String, Bytes, Array, Map)
- Binary codec (55% smaller than MessagePack)
- Address parsing and pattern matching
- Frame format and QoS

```rust
use clasp_core::{Message, SetMessage, Value, codec};

let msg = Message::Set(SetMessage {
    address: "/lights/brightness".to_string(),
    value: Value::Float(0.75),
    revision: None,
    lock: false,
    unlock: false,
});

let bytes = codec::encode(&msg)?;
```

### clasp-transport

**Role:** Network I/O  
**Dependencies:** clasp-core  
**Features:** websocket, quic, udp, ble, serial, webrtc

Contains transport implementations:

| Transport | Use Case | Platform |
|-----------|----------|----------|
| WebSocket | Universal, browsers | All |
| QUIC | High-performance native | Desktop, Server |
| UDP | Low latency, local | Desktop, Server |
| BLE | Short range IoT | Desktop, Mobile |
| Serial | Hardware control | Desktop |
| WebRTC | P2P, NAT traversal | All |

```rust
use clasp_transport::websocket;

// Client
let conn = websocket::connect("wss://relay.clasp.to").await?;

// Server
websocket::serve("0.0.0.0:7330", handler).await?;
```

### clasp-router

**Role:** Message routing and state management  
**Dependencies:** clasp-core, clasp-transport  
**This is the ROUTER library.**

Use when you want to:
- Build a CLASP router (central message hub)
- Route messages between clients
- Manage state with revisions
- Handle subscriptions

```rust
use clasp_router::{Router, RouterConfig};

let router = Router::new(RouterConfig::default());
router.serve_websocket("0.0.0.0:7330").await?;
```

### clasp-client

**Role:** Client library  
**Dependencies:** clasp-core, clasp-transport

Use when you want to:
- Connect to a CLASP server
- Subscribe to addresses
- Publish/set values

```rust
use clasp_client::Clasp;

let client = Clasp::connect("wss://relay.clasp.to").await?;
client.set("/my/value", 42.0).await?;
client.subscribe("/my/**", |value, addr| {
    println!("{} = {:?}", addr, value);
}).await?;
```

### clasp-bridge

**Role:** Protocol translation  
**Dependencies:** clasp-core

Bridges external protocols to/from CLASP:

| Bridge | External Protocol | Direction |
|--------|-------------------|-----------|
| OSC | Open Sound Control | Bidirectional |
| MIDI | Musical Instruments | Bidirectional |
| DMX | Lighting (DMX512) | Output |
| Art-Net | Lighting over IP | Bidirectional |
| MQTT | IoT messaging | Bidirectional |
| sACN | Streaming ACN | Output |

**A bridge connects as a CLIENT to a CLASP router.** It does not route messages itself.

```
┌─────────────┐      ┌──────────────┐      ┌─────────────┐
│  TouchOSC   │ OSC  │  OSC Bridge  │CLASP │   Router    │
│   (iPad)    │ ───► │  (client)    │ ───► │  (server)   │
└─────────────┘      └──────────────┘      └─────────────┘
```

### clasp-discovery

**Role:** Service discovery  
**Dependencies:** clasp-core

Finds CLASP servers on the network:
- **mDNS:** `_clasp._tcp.local` (standard DNS-SD)
- **UDP Broadcast:** Port 7331 (for networks without mDNS)

```rust
use clasp_discovery::mdns::ServiceBrowser;

let browser = ServiceBrowser::new()?;
browser.on_found(|service| {
    println!("Found: {} at {}", service.name, service.address);
});
```

### clasp-embedded

**Role:** Minimal no_std implementation  
**Dependencies:** None  
**Size:** ~3.6KB RAM

For microcontrollers (ESP32, RP2040, STM32, etc.).

**Important:** This crate is transport-agnostic. YOU provide the bytes.

```rust
use clasp_embedded::{Client, Value, encode_set_frame};

let mut client = Client::new();

// Prepare a SET frame
let frame = client.prepare_set("/sensor/temp", Value::Float(25.5));

// Send via YOUR transport (WebSocket, HTTP POST, UDP, BLE, etc.)
your_transport.send(frame);
```

See [Embedded Transports](#embedded-transports) for connection options.

### clasp-wasm

**Role:** Browser bindings  
**Dependencies:** clasp-core, clasp-client

WebAssembly build of the client for browsers. WebSocket only.

## Common Patterns

### Pattern 1: Simple Client

Connect to a public relay:

```rust
let client = Clasp::connect("wss://relay.clasp.to").await?;
client.set("/hello", "world").await?;
```

### Pattern 2: Local Router + Clients

Run your own router:

```rust
// Router (one process)
let router = Router::new(config);
router.serve_websocket("0.0.0.0:7330").await?;

// Clients (other processes)
let client = Clasp::connect("ws://localhost:7330").await?;
```

### Pattern 3: Embedded Server

Embed CLASP in your application:

```rust
let router = Arc::new(Router::new(config));
let r = router.clone();

// Your app logic publishes to CLASP
tokio::spawn(async move {
    loop {
        let temp = read_sensor();
        r.state().set_value("/sensor/temp", Value::Float(temp), "server");
        sleep(Duration::from_secs(1)).await;
    }
});

// CLASP router accepts connections
router.serve_websocket("0.0.0.0:7330").await?;
```

### Pattern 4: Bridge Setup

Connect external protocols:

```
┌──────────────────────────────────────────────────────────────┐
│                      CLASP Router                             │
│                    (clasp-router)                             │
└─────────────────────────┬────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
   ┌────▼────┐       ┌────▼────┐       ┌────▼────┐
   │  OSC    │       │  MIDI   │       │  MQTT   │
   │ Bridge  │       │ Bridge  │       │ Bridge  │
   └────┬────┘       └────┬────┘       └────┬────┘
        │                 │                 │
   ┌────▼────┐       ┌────▼────┐       ┌────▼────┐
   │TouchOSC │       │Ableton  │       │Sensors  │
   └─────────┘       └─────────┘       └─────────┘
```

## Embedded Transports

`clasp-embedded` doesn't include transport code. Here's how to connect:

### HTTP POST (Simplest)

```rust
let frame = client.prepare_set("/sensor/temp", Value::Float(25.5));
http_client.post("https://relay.clasp.to/frames", frame);
```

### WebSocket (Bidirectional)

```rust
// Connect WebSocket (platform-specific)
let ws = WebSocket::connect("wss://relay.clasp.to");

// Send CLASP frames
ws.send(client.prepare_hello("ESP32"));
ws.send(client.prepare_set("/sensor/temp", Value::Float(25.5)));

// Receive
if let Some(data) = ws.receive() {
    if let Some(msg) = client.process(data) {
        // Handle message
    }
}
```

### UDP (Local Network)

```rust
let socket = UdpSocket::bind("0.0.0.0:0")?;
socket.send_to(frame, "192.168.1.100:7331")?;
```

### MQTT (Through Broker)

Use MQTT as transport, with CLASP frames as payload:

```rust
mqtt.publish("clasp/frames", frame)?;
```

## When to Use What

| Scenario | Use |
|----------|-----|
| Browser app | `@clasp-to/core` (JS) or `clasp-wasm` |
| Desktop app | `clasp-client` |
| Custom server | `clasp-router` |
| Protocol bridge | `clasp-bridge` + `clasp-client` |
| ESP32/microcontroller | `clasp-embedded` |
| Cloud deployment | `clasp-router` via Docker |

## Feature Flags

Most crates use feature flags to minimize binary size:

### clasp-transport

```toml
[dependencies]
clasp-transport = { version = "3.1", features = ["websocket"] }
# Or for all transports:
clasp-transport = { version = "3.1", features = ["full"] }
```

### clasp-router

```toml
[dependencies]
clasp-router = { version = "3.1", features = ["websocket"] }
```

### clasp-embedded

```toml
[dependencies]
clasp-embedded = { version = "3.1", features = ["client"] }
# Or for server mode:
clasp-embedded = { version = "3.1", features = ["server"] }
```
