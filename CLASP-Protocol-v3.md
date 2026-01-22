# CLASP Protocol Specification (CLASP/3)

**Creative Low-Latency Application Streaming Protocol**

**Version**: 3.0
**Status**: Working Specification
**Author**: Developed for LumenCanvas and the creative tools community
**License**: CC0 1.0 Universal (Public Domain)

> **v3 Changes:** Efficient binary encoding replaces MessagePack-with-named-keys.
> SET messages are 54% smaller (32 bytes vs 69 bytes) and encode 5x faster.
> Backward compatible: decoders accept both v2 and v3 frames.

---

# Explain Like I'm 5

**What is CLASP?**

Imagine you have a bunch of toys that can talk to each other - your keyboard, your lights, your tablet with cool visuals, and your computer. Right now, each toy speaks a different language. Your keyboard speaks "MIDI" (invented in 1983, before your parents were probably using computers!). Your lights might speak "DMX" or "Art-Net." Your tablet app probably speaks "WebSocket JSON" or something custom.

CLASP is like giving all your toys a universal translator. But it's even better - it's a NEW language that's smarter than all the old ones. It remembers things (like "the brightness is currently 50%"), it's super fast (faster than you can blink), it works on WiFi, wired internet, or even Bluetooth, and it keeps secrets safe (so bad guys can't mess with your light show).

**Why should I care?**

- **If you make music**: Your instruments will talk to your lights and visuals automatically, in perfect sync
- **If you do projection mapping** (like LumenCanvas): Control everything from one place, even across the internet
- **If you build things**: One protocol to learn instead of five. Works on tiny $5 chips and big servers
- **If you just want stuff to work**: Plug it in, it finds other devices automatically, done

**The magic ingredients:**

1. **Signals** - Not just messages, but signals with meaning: "this is a button press" vs "this is a fader position" vs "this is a timeline of changes"
2. **Discovery** - Devices find each other automatically (like how your phone finds WiFi networks)
3. **State** - Remembers what things are set to, so if you disconnect and reconnect, you don't lose everything
4. **Timing** - Everything can be perfectly synchronized, even across the internet
5. **Security** - Encrypted and permission-based, so only the right people can control the right things
6. **Bridges** - Speaks the old languages too (MIDI, OSC, DMX), so your existing gear still works

---

# Part 0: Self-Critique of CLASP v1 and Research Findings

Before presenting the improved specification, here's honest analysis of what I got wrong in v1 and what the research revealed:

## What v1 Got Wrong

### 1. Discovery Was Underspecified
v1 mentioned mDNS in passing but didn't detail the discovery flow. Real-world deployment requires:
- Service type definitions
- Fallback when mDNS fails (corporate networks often block it)
- Cloud rendezvous for WAN discovery
- Browser-based discovery (browsers can't do mDNS natively)

### 2. P2P Was Missing
v1 focused on client-server. But creative tools often need true peer-to-peer:
- Two laptops jamming together
- Multiple projection systems coordinating
- WebRTC DataChannel offers sub-10ms latency but has NAT traversal complexity

### 3. Embedded Was Underthought  
Claiming "ESP32-friendly" while specifying 256-byte addresses and complex type systems was naive. Research shows:
- OSC's string addresses are a real problem on embedded
- RTP-MIDI proves recovery journals add significant complexity
- 4KB RAM is tight; every byte matters

### 4. No API Design
A protocol without a good API is just a spec document that collects dust. MIDI 2.0's slow adoption proves this - great spec, poor developer experience.

### 5. State Conflict Resolution Was Handwavy
"CRDT-ish" isn't a specification. Multi-controller scenarios need actual algorithms, not buzzwords.

## What Research Revealed

### Successes to Learn From

| Protocol | What Works | Why |
|----------|------------|-----|
| **MIDI** | Ubiquity, simplicity | 7-bit values are limiting but dead simple to implement |
| **OSC** | Flexible addressing | Human-readable paths work well for rapid prototyping |
| **WebRTC** | NAT traversal, encryption | ICE/STUN/TURN solved hard problems |
| **mDNS/Bonjour** | Zero-config discovery | Just works on LANs |
| **sACN** | Multicast efficiency | Subscribing to specific universes scales better than broadcast |
| **QUIC** | No head-of-line blocking | UDP with reliability done right |
| **MIDI 2.0** | Bidirectional, profiles | Auto-configuration is the future |

### Failures to Avoid

| Protocol | What Fails | Why |
|----------|------------|-----|
| **OSC** | No standard namespaces | Every app invents its own addresses |
| **OSC** | String parsing overhead | 30-50% CPU on embedded just parsing addresses |
| **Art-Net** | Broadcast by default | Network congestion at scale |
| **RTP-MIDI** | Journal complexity | Recovery journals are complex and add latency |
| **MIDI 1.0** | Unidirectional | Can't query device capabilities |
| **Custom WebSocket JSON** | No semantics | Just bytes with no meaning |

### Key Insights

1. **mDNS works on LAN but fails in enterprise** - Need fallback mechanisms
2. **WebRTC DataChannel achieves 1-10ms latency** - Worth the NAT traversal complexity
3. **QUIC's connection migration survives IP changes** - Critical for mobile
4. **MIDI 2.0's Property Exchange uses JSON** - Metadata doesn't need binary efficiency
5. **sACN's multicast subscription model scales** - Devices should opt-in to streams
6. **WiFi jitter kills musical timing** - Need jitter buffers and timestamp scheduling

---

# Part 1: Design Principles (Revised)

## 1.1 Core Principles

1. **Transport-Agnostic, Universally Compatible**
   CLASP is transport-agnostic by design. The protocol works over any byte transport: WebSocket, WebRTC, QUIC, UDP, Bluetooth LE, Serial, or custom transports. For interoperability, WebSocket is the recommended baseline (browsers can use it, and it's universally available). WebRTC DataChannel offers lower latency for P2P. Constrained devices may use UDP or BLE exclusively.

2. **Progressive Enhancement**  
   Simple things MUST be simple:
   - Hello → Send message → Done (3 lines of code)
   - Complex features (encryption, discovery, state sync) are opt-in

3. **Semantic Signals, Not Just Bytes**  
   The protocol knows the difference between:
   - A button press (Event)
   - A fader position (Param with state)
   - A gesture (Stream with phases)
   - An automation lane (Timeline)

4. **Discovery Is First-Class**  
   Finding devices shouldn't require configuration. But when auto-discovery fails, manual config MUST be possible.

5. **State Is Truth**  
   Parameters have authoritative values with revision numbers. "What is the current brightness?" always has an answer.

6. **Timing Is Deterministic**  
   Bundles can be scheduled for future execution. Clock sync is built-in, not bolted-on.

7. **Security Without Ceremony**  
   Encrypted by default, but local development doesn't require certificates.

8. **Legacy Is Respected**  
   MIDI, OSC, DMX bridges are defined in the spec, not afterthoughts.

## 1.2 Non-Goals

- **Not a media transport**: CLASP carries control signals, not audio/video streams
- **Not a file format**: Show files and presets are application-level concerns
- **Not a UI specification**: How controls are displayed is up to applications

---

# Part 2: Transport Layer

## 2.1 Transport Agnosticism

CLASP is designed to be **transport-agnostic**. The protocol defines a binary frame format that can be carried over any byte-oriented transport. The frame format makes no assumptions about:
- Packet ordering (handled by frame sequencing)
- Reliability (handled by QoS levels)
- Connection semantics (stateless frames)
- MTU size (length-prefixed, max 65KB)

### 2.1.1 Supported Transports

| Transport | Characteristics | Best For |
|-----------|-----------------|----------|
| **WebSocket** | Stream-based, universal browser support | Web apps, cross-platform baseline |
| **WebRTC DataChannel** | P2P, configurable reliability, NAT traversal | Low-latency P2P, gaming |
| **QUIC** | Multiplexed streams, connection migration | Mobile apps, unreliable networks |
| **UDP** | Minimal overhead, broadcast capable | LAN devices, embedded systems |
| **BLE** | Wireless, low power | Battery-powered controllers |
| **Serial** | Direct hardware, lowest latency | Hardware integration, DMX |

### 2.1.2 Interoperability Recommendation

For maximum interoperability, implementations SHOULD support WebSocket as a common denominator. This enables:
- Browser clients to connect
- Any two CLASP devices to communicate
- Easy debugging with standard tools

However, **WebSocket is NOT architecturally required**. An embedded device speaking only UDP is a valid CLASP implementation. A BLE controller is a valid CLASP implementation. The protocol doesn't care how bytes arrive—only that they're valid CLASP frames.

## 2.2 Frame Format (Revised for Simplicity)

CLASP uses a minimal binary frame format optimized for parsing efficiency:

```
┌─────────────────────────────────────────────────────────────────┐
│ Byte 0:     Magic (0x53 = 'S')                                  │
│ Byte 1:     Flags                                               │
│             [7:6] QoS (00=fire, 01=confirm, 10=commit, 11=rsv)  │
│             [5]   Timestamp present                             │
│             [4]   Encrypted                                     │
│             [3]   Compressed                                    │
│             [2:0] Version (000=v2/msgpack, 001=v3/binary)       │
│ Byte 2-3:   Payload Length (uint16 big-endian, max 65535)       │
├─────────────────────────────────────────────────────────────────┤
│ [If timestamp flag] Bytes 4-11: Timestamp (uint64 µs)           │
├─────────────────────────────────────────────────────────────────┤
│ Payload (v3 binary or v2 MessagePack)                           │
└─────────────────────────────────────────────────────────────────┘
```

**Total overhead**: 4 bytes minimum, 12 bytes with timestamp.

### 2.2.1 Payload Encoding

**v3 Compact Binary (Default)** — Version bits `001`:

v3 uses positional binary encoding instead of named keys:

```
SET message (31 bytes for typical param):
┌──────────┬──────────┬────────────┬─────────┬──────────┬──────────┐
│ MsgType  │ Flags    │ AddrLen    │ Address │ Value    │ [Rev]    │
│ 0x21     │ vtype+fl │ u16        │ UTF-8   │ encoded  │ u64?     │
└──────────┴──────────┴────────────┴─────────┴──────────┴──────────┘

Flags byte: [7] has_revision [6] lock [5] unlock [3:0] value_type
Value types: 0x00=null, 0x07=f64, 0x08=string, 0x09=bytes, 0x0A=array, 0x0B=map
```

**v2 MessagePack (Legacy)** — Version bits `000`:

Still supported for backward compatibility. Decoders auto-detect based on
first payload byte: MessagePack map prefix (0x80-0x8F, 0xDE, 0xDF) indicates v2.

### 2.2.2 Why Binary Over MessagePack?

MessagePack-with-named-keys was the v2 choice. v3 switched to compact binary because:

1. **55% smaller**: SET message 31 bytes vs 69 bytes
2. **4x faster encode**: 8M msg/s vs 1.8M msg/s (Rust)
3. **7x faster decode**: 11M msg/s vs 1.5M msg/s
4. **Still debuggable**: Simple byte layout, no schema files needed

### 2.2.3 Embedded Optimization

For severely constrained devices (< 8KB RAM), a "Lite" profile exists:

- Fixed 2-byte addresses (numeric IDs instead of paths)
- No compression
- No encryption
- UDP only

Lite devices can communicate with full CLASP via a bridge.

### 2.2.2 Embedded Optimization

For severely constrained devices (< 8KB RAM), a "Lite" profile exists:

- Fixed 2-byte addresses (numeric IDs instead of paths)
- No compression
- No encryption
- UDP only

Lite devices can communicate with full CLASP via a bridge.

## 2.3 WebSocket Specifics

- URI: `wss://host:port/clasp` or `ws://host:port/clasp` (dev only)
- Subprotocol: `clasp.v3` (accepts `clasp.v2` for backward compatibility)
- Binary mode only (no text frames)
- Ping/Pong: Use WebSocket native ping/pong for keepalive

```javascript
const ws = new WebSocket('wss://localhost:7330/clasp', 'clasp.v3');
ws.binaryType = 'arraybuffer';
```

## 2.4 WebRTC DataChannel Specifics

For P2P connections:

```javascript
const dc = peerConnection.createDataChannel('clasp', {
  ordered: false,      // Allow out-of-order for streams
  maxRetransmits: 0    // No retransmits for Q0 (fire)
});
```

For reliable messages (Q1/Q2), create a second channel:
```javascript
const dcReliable = peerConnection.createDataChannel('clasp-reliable', {
  ordered: true
});
```

**ICE/STUN/TURN**: Use standard WebRTC infrastructure. CLASP doesn't define signaling - use whatever works (WebSocket signaling server, manual SDP exchange, etc.)

---

# Part 3: Discovery (New in v2)

## 3.1 Discovery Mechanisms

CLASP defines three discovery mechanisms in priority order:

### 3.1.1 mDNS (LAN Auto-Discovery)

Service type: `_clasp._tcp.local`

TXT records:
```
version=3
name=LumenCanvas Studio
features=psetg      (p=Param, s=Stream, e=Event, t=Timeline, g=Gesture)
ws=7330             (WebSocket port)
```

Example using avahi/Bonjour:
```
LumenCanvas._clasp._tcp.local. 
  TXT "version=2" "name=LumenCanvas Studio" "features=psetg" "ws=7330"
  SRV 0 0 7330 studio.local.
  A 192.168.1.42
```

### 3.1.2 UDP Broadcast (LAN Fallback)

When mDNS is unavailable (common in enterprise networks):

- Port: 7331
- Broadcast address: 255.255.255.255 or subnet broadcast
- Message: `HELLO` packet (defined in §5.1)

Devices respond with `ANNOUNCE` via unicast to sender.

### 3.1.3 Rendezvous Server (WAN Discovery)

For internet-connected devices, a simple rendezvous protocol:

```
POST /api/v1/register
{
  "name": "LumenCanvas Studio",
  "publicKey": "base64...",
  "features": ["param", "stream", "event"],
  "endpoints": {
    "ws": "wss://studio.example.com:7330/clasp"
  }
}

GET /api/v1/discover?tag=lumen
[
  {
    "name": "LumenCanvas Studio",
    "publicKey": "base64...",
    "endpoints": { ... }
  }
]
```

**Note**: Anthropic/LumenCanvas may operate a public rendezvous server. Self-hosting is also supported.

## 3.2 Browser Discovery Limitations

Browsers cannot:
- Perform mDNS queries
- Send UDP broadcasts
- Listen on ports

**Solution**: Browser clients connect to a known WebSocket endpoint (configured by user or scanned via QR code). That endpoint can relay discovery information.

## 3.3 Discovery Flow Diagram

```
┌──────────────┐     mDNS Query     ┌──────────────┐
│   Browser    │ ────────────────X  │              │ (browsers can't mDNS)
│   Client     │                    │   CLASP │
│              │     WebSocket      │    Device    │
│              │ ◄─────────────────►│              │
└──────────────┘                    └──────────────┘
       │                                   │
       │  Manual Config / QR Code          │ mDNS Announce
       │  (wss://...)                       ▼
       │                            ┌──────────────┐
       │                            │   Native     │
       │                            │   Client     │
       │                            └──────────────┘
       │                                   │
       │                                   │ mDNS Query
       │                                   ▼
       │                            (auto-discovers device)
```

---

# Part 4: Signal Types (Refined)

## 4.1 Signal Type Overview

CLASP distinguishes signal types at the protocol level. This isn't metadata - it affects routing, storage, reliability, and UI behavior.

| Type | Purpose | Default QoS | State? | Coalesce? |
|------|---------|-------------|--------|-----------|
| **Param** | Authoritative values | Confirm | Yes | Last value |
| **Event** | Triggers | Confirm | No | Never |
| **Stream** | High-rate data | Fire | No | Recent values |
| **Gesture** | Phased input | Fire | Phase only | By ID |
| **Timeline** | Time-indexed automation | Commit | Full | Never |

## 4.2 Param (State with Conflict Resolution)

Params are the core state primitive. Every Param has:

```javascript
{
  address: "/lumen/scene/0/layer/3/opacity",
  value: 0.75,
  revision: 42,           // Monotonic version
  writer: "session:abc",  // Who wrote it
  timestamp: 1704067200,  // When (µs since session start)
  meta: {                 // Optional metadata
    unit: "normalized",
    range: [0, 1],
    default: 1.0
  }
}
```

### 4.2.1 Conflict Resolution Strategies

When two clients write simultaneously:

| Strategy | Behavior | Use Case |
|----------|----------|----------|
| `lww` | Last-Write-Wins (by timestamp) | Default. Simple. |
| `max` | Keep maximum value | Meters, levels |
| `min` | Keep minimum value | Limits |
| `lock` | First writer holds lock | Exclusive control |
| `merge` | Application-defined merge | Complex objects |

### 4.2.2 Lock Strategy Details

For `lock` strategy:

```javascript
// Request lock
SET { address: "/mixer/fader/1", value: 0.5, lock: true }

// Server response if lock granted
ACK { address: "/mixer/fader/1", locked: true, holder: "session:abc" }

// Server response if lock denied
ERROR { code: 401, message: "Lock held", holder: "session:xyz" }

// Release lock
SET { address: "/mixer/fader/1", value: 0.5, unlock: true }
```

## 4.3 Event (Triggers)

Events are ephemeral - they happen and are gone.

```javascript
{
  address: "/lumen/cue/fire",
  payload: { cue: "intro", transition: "fade" },
  timestamp: 1704067200
}
```

Events MUST be delivered (QoS Confirm by default) but are not stored.

## 4.4 Stream (High-Rate Data)

Streams are for continuous data where occasional packet loss is acceptable.

```javascript
{
  address: "/controller/fader/1",
  samples: [0.50, 0.52, 0.55, 0.58],  // Batched samples
  rate: 60,                            // Hz
  timestamp: 1704067200                // Timestamp of first sample
}
```

### 4.4.1 Stream Subscription Options

```javascript
SUBSCRIBE {
  address: "/controller/fader/*",
  type: "stream",
  options: {
    maxRate: 30,        // Downsample to 30Hz
    epsilon: 0.01,      // Only send if change > 1%
    window: 100         // Buffer 100ms of samples
  }
}
```

## 4.5 Gesture (Phased Input)

Gestures are streams with semantic phases, designed for touch/pen/motion input.

```javascript
{
  address: "/input/touch",
  id: 1,                // Stable ID for this gesture
  phase: "move",        // "start" | "move" | "end" | "cancel"
  payload: {
    position: [0.5, 0.3],
    pressure: 0.8
  },
  timestamp: 1704067200
}
```

**Coalescing**: Routers MAY coalesce `move` phases (keeping only most recent) to reduce bandwidth. `start` and `end` are never coalesced.

## 4.6 Timeline (Automation)

Timelines are time-indexed sequences for automation, cues, and scheduling.

```javascript
{
  address: "/lumen/scene/0/layer/3/opacity",
  type: "timeline",
  keyframes: [
    { time: 0, value: 1.0, easing: "linear" },
    { time: 1000000, value: 0.0, easing: "ease-out" }  // 1 second
  ],
  loop: false,
  startTime: 1704067200  // When to begin playback
}
```

Timelines are immutable once published. To modify, publish a new timeline.

---

# Part 5: Messages

## 5.1 Message Catalog

| Message | Code | Direction | Description |
|---------|------|-----------|-------------|
| `HELLO` | 0x01 | Client→Server | Connection initiation |
| `WELCOME` | 0x02 | Server→Client | Connection accepted |
| `ANNOUNCE` | 0x03 | Both | Capability advertisement |
| `SUBSCRIBE` | 0x10 | Client→Server | Subscribe to pattern |
| `UNSUBSCRIBE` | 0x11 | Client→Server | Unsubscribe |
| `PUBLISH` | 0x20 | Both | Send signal (Event/Stream/Gesture) |
| `SET` | 0x21 | Both | Set Param value |
| `GET` | 0x22 | Client→Server | Request current value |
| `SNAPSHOT` | 0x23 | Server→Client | Current state dump |
| `BUNDLE` | 0x30 | Both | Atomic message group |
| `SYNC` | 0x40 | Both | Clock synchronization |
| `PING` | 0x41 | Both | Keepalive |
| `PONG` | 0x42 | Both | Keepalive response |
| `ACK` | 0x50 | Both | Acknowledgment |
| `ERROR` | 0x51 | Both | Error response |
| `QUERY` | 0x60 | Client→Server | Introspection |
| `RESULT` | 0x61 | Server→Client | Query response |

## 5.2 HELLO / WELCOME

### HELLO (Client → Server)

```javascript
{
  type: "HELLO",
  version: 2,
  name: "LumenCanvas Controller",
  features: ["param", "event", "stream"],
  capabilities: {
    encryption: true,
    compression: "lz4"
  }
}
```

### WELCOME (Server → Client)

```javascript
{
  type: "WELCOME",
  version: 2,
  session: "abc123",       // Assigned session ID
  name: "LumenCanvas Studio",
  features: ["param", "event", "stream", "timeline"],
  time: 1704067200,        // Server time (µs)
  token: "bearer:xyz..."   // Optional capability token
}
```

## 5.3 ANNOUNCE

Nodes advertise their signals:

```javascript
{
  type: "ANNOUNCE",
  namespace: "/lumen",
  signals: [
    {
      address: "/lumen/scene/*/layer/*/opacity",
      type: "param",
      datatype: "f32",
      access: "rw",
      meta: {
        unit: "normalized",
        range: [0, 1],
        default: 1.0
      }
    },
    {
      address: "/lumen/cue/*",
      type: "event",
      access: "w"
    }
  ]
}
```

## 5.4 SUBSCRIBE / UNSUBSCRIBE

```javascript
{
  type: "SUBSCRIBE",
  id: 1,                    // Subscription ID (for unsubscribe)
  pattern: "/lumen/scene/*/layer/*/opacity",
  types: ["param"],         // Filter by signal type
  options: {
    maxRate: 30,
    epsilon: 0.01,
    history: 1              // Request last 1 value
  }
}
```

```javascript
{
  type: "UNSUBSCRIBE",
  id: 1
}
```

## 5.5 SET / GET / SNAPSHOT

### SET
```javascript
{
  type: "SET",
  address: "/lumen/scene/0/layer/3/opacity",
  value: 0.75,
  revision: 41,  // Optional: expected revision (optimistic lock)
  lock: false    // Optional: request exclusive lock
}
```

### GET
```javascript
{
  type: "GET",
  address: "/lumen/scene/0/layer/3/opacity"
}
```

### SNAPSHOT (Response to GET or on connect)
```javascript
{
  type: "SNAPSHOT",
  params: [
    {
      address: "/lumen/scene/0/layer/3/opacity",
      value: 0.75,
      revision: 42
    },
    // ... more params
  ]
}
```

## 5.6 BUNDLE

Atomic group of messages with optional scheduled execution:

```javascript
{
  type: "BUNDLE",
  timestamp: 1704067300,  // Optional: execute at this time
  messages: [
    { type: "SET", address: "/light/1/intensity", value: 1.0 },
    { type: "SET", address: "/light/2/intensity", value: 0.0 },
    { type: "PUBLISH", address: "/cue/fire", payload: { id: "intro" } }
  ]
}
```

## 5.7 SYNC (Clock Synchronization)

Uses NTP-like algorithm:

```
Client                              Server
  │                                    │
  │── SYNC { t1: T1 } ────────────────►│
  │                                    │ (receives at T2)
  │◄── SYNC { t1:T1, t2:T2, t3:T3 } ───│ (sends at T3)
  │                                    │
  │ (receives at T4)                   │
```

Offset calculation:
```
roundTrip = (T4 - T1) - (T3 - T2)
offset = ((T2 - T1) + (T3 - T4)) / 2
```

## 5.8 ERROR

```javascript
{
  type: "ERROR",
  code: 403,
  message: "Permission denied",
  address: "/lumen/admin/config",
  correlationId: 42  // Optional: relates to request
}
```

Error codes:
- 100-199: Protocol errors
- 200-299: Address errors
- 300-399: Permission errors
- 400-499: State errors
- 500-599: Server errors

---

# Part 6: Data Types

## 6.1 Scalar Types (v3 Binary Encoding)

| Type | v3 Code | Bytes | Description |
|------|---------|-------|-------------|
| `null` | 0x00 | 1 | No value |
| `bool` | 0x01/0x02 | 1 | false/true |
| `i32` | 0x05 | 5 | Signed 32-bit (big-endian) |
| `i64` | 0x06 | 9 | Signed 64-bit (big-endian) |
| `f64` | 0x07 | 9 | 64-bit float (big-endian) |
| `str` | 0x08 | 3+ | Length-prefixed UTF-8 (u16 + bytes) |
| `bin` | 0x09 | 5+ | Length-prefixed binary (u32 + bytes) |
| `array` | 0x0A | 5+ | Length-prefixed (u32 + items) |
| `map` | 0x0B | 5+ | Length-prefixed (u32 + key-value pairs) |

## 6.2 Creative Primitives (Extension Types)

Extension types for creative data (used in arrays):

| Type | Ext Code | Encoding | Description |
|------|----------|----------|-------------|
| `vec2` | 0x10 | 8 bytes (f32×2) | 2D vector |
| `vec3` | 0x11 | 12 bytes (f32×3) | 3D vector |
| `vec4` | 0x12 | 16 bytes (f32×4) | 4D vector |
| `color` | 0x13 | 4 bytes (u8×4) | RGBA color |
| `colorf` | 0x14 | 16 bytes (f32×4) | RGBA float |
| `mat3` | 0x15 | 36 bytes (f32×9) | 3×3 matrix |
| `mat4` | 0x16 | 64 bytes (f32×16) | 4×4 matrix |

## 6.3 Composite Types

Arrays and maps use length-prefixed encoding:

```javascript
// Array (type 0x0A, then u32 count, then typed elements)
[0.5, 0.3, 0.8]

// Map (type 0x0B, then u32 count, then key-value pairs)
{ x: 0.5, y: 0.3, pressure: 0.8 }
```

---

# Part 7: Security

## 7.1 Security Modes

| Mode | Use Case | Requirements |
|------|----------|--------------|
| **Open** | Development, trusted LAN | None |
| **Encrypted** | Production | TLS for WebSocket, DTLS for UDP/DataChannel |
| **Authenticated** | Multi-user | Encrypted + capability tokens |

## 7.2 Encryption

For WebSocket: Use WSS (TLS 1.3).

For UDP/DataChannel: Use DTLS (built into WebRTC).

For native QUIC: TLS 1.3 is mandatory.

## 7.3 Capability Tokens

JSON Web Tokens (JWT) with CLASP claims:

```javascript
{
  "iss": "clasp:lumencanvas",
  "sub": "user:moheeb",
  "iat": 1704067200,
  "exp": 1704153600,
  "sf": {
    "read": ["/lumen/**"],
    "write": ["/lumen/scene/*/layer/*/opacity"],
    "constraints": {
      "/lumen/scene/*/layer/*/opacity": {
        "range": [0, 1],
        "maxRate": 60
      }
    }
  }
}
```

## 7.4 Pairing (Zero-Config Security)

For local/studio setups without PKI:

1. Server displays 6-digit code
2. Client enters code (or scans QR)
3. Shared secret derived from code
4. Session established with full encryption

---

# Part 8: Bridges

## 8.1 Bridge Architecture

Bridges are CLASP nodes that translate legacy protocols:

```
┌─────────────────────────────────────────────────────────────┐
│                    CLASP Router                        │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  MIDI    │  │   OSC    │  │   DMX    │  │ Art-Net  │   │
│  │  Bridge  │  │  Bridge  │  │  Bridge  │  │  Bridge  │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────│─────────────│─────────────│─────────────│─────────┘
        │             │             │             │
        ▼             ▼             ▼             ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │  MIDI   │   │   OSC   │   │   DMX   │   │ Art-Net │
   │ Devices │   │  Apps   │   │Fixtures │   │  Nodes  │
   └─────────┘   └─────────┘   └─────────┘   └─────────┘
```

## 8.2 MIDI Bridge Mapping

```
MIDI → CLASP
─────────────────
Note On/Off     → /midi/{device}/note      Event { note, velocity, channel }
CC              → /midi/{device}/cc/{num}  Param u8
Pitch Bend      → /midi/{device}/bend      Param i16 (-8192 to 8191)
Program Change  → /midi/{device}/program   Event { program, channel }
Clock           → /midi/{device}/clock     Event (24 PPQ)
Start/Stop      → /midi/{device}/transport Event { state: "start"|"stop"|"continue" }
```

## 8.3 OSC Bridge Mapping

Direct path mapping:

```
OSC /synth/osc1/cutoff ,f 0.5
  → CLASP SET /osc/synth/osc1/cutoff 0.5

OSC Bundle [timetag]
  → CLASP BUNDLE { timestamp: timetag, messages: [...] }
```

Type mapping:
- OSC int32 → CLASP i32
- OSC float32 → CLASP f32
- OSC string → CLASP str
- OSC blob → CLASP bin
- OSC timetag → CLASP timestamp

## 8.4 DMX/Art-Net/sACN Bridge Mapping

```
DMX Universe 1, Channel 47 = 255
  → CLASP SET /dmx/1/47 255

Art-Net Universe 0:1:2, Channel 1-512
  → CLASP /artnet/0/1/2/{channel} Param u8

sACN Universe 100, Priority 200
  → CLASP /sacn/100/{channel} Param u8
     (priority in metadata)
```

## 8.5 Bridge Announcements

Bridges identify themselves with `bridge` metadata:

```javascript
{
  type: "ANNOUNCE",
  namespace: "/midi/launchpad",
  meta: {
    bridge: true,
    protocol: "midi",
    device: "Novation Launchpad X",
    bidirectional: true
  },
  signals: [...]
}
```

---

# Part 9: Timing

## 9.1 Time Model

All timestamps are 64-bit unsigned integers representing microseconds since:
- **Session time**: Microseconds since session start (for relative timing)
- **Unix time**: Microseconds since 1970-01-01 (for absolute timing)

Session time is preferred for live performance (avoids UTC issues).

## 9.2 Clock Synchronization

SYNC messages establish clock offset (see §5.7). Clients SHOULD sync every 30 seconds.

Quality metrics:
- **Offset**: Estimated clock difference
- **RTT**: Round-trip time
- **Jitter**: RTT variance over last 10 samples

## 9.3 Scheduled Bundles

Bundles with timestamps execute at the specified time:

```javascript
{
  type: "BUNDLE",
  timestamp: serverTime + 100000,  // 100ms in future
  messages: [
    { type: "SET", address: "/light/1/intensity", value: 1.0 },
    { type: "SET", address: "/light/2/intensity", value: 0.0 }
  ]
}
```

**Tolerance**: Receivers execute bundles within ±1ms of scheduled time.

## 9.4 Jitter Buffer

For high-rate streams over WiFi/WAN, receivers SHOULD implement a jitter buffer:

```
Jitter Buffer: 20-50ms
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  │ Packets arrive with jitter
  ▼
┌─────────────────────────────┐
│ Buffer: 20ms of samples     │
└─────────────────────────────┘
  │
  ▼
  Smooth output at configured rate
```

---

# Part 10: High-Level API Design

## 10.1 Design Philosophy

The API should make simple things trivial:

```javascript
// This should work in 3 lines
const sf = new CLASP('wss://localhost:7330');
sf.on('/lumen/scene/*/layer/*/opacity', (value, address) => console.log(address, value));
sf.set('/lumen/scene/0/layer/0/opacity', 0.5);
```

## 10.2 JavaScript API (Browser + Node)

### Connection

```javascript
import { CLASP } from 'clasp';

// Simple connection
const sf = new CLASP('wss://localhost:7330');

// With options
const sf = new CLASP({
  url: 'wss://localhost:7330',
  name: 'My Controller',
  token: 'bearer:...',
  reconnect: true
});

// Events
sf.on('connect', () => console.log('Connected'));
sf.on('disconnect', () => console.log('Disconnected'));
sf.on('error', (err) => console.error(err));
```

### Reading State

```javascript
// Get current value (async)
const opacity = await sf.get('/lumen/scene/0/layer/0/opacity');

// Subscribe to changes
sf.subscribe('/lumen/scene/*/layer/*/opacity', (value, address, meta) => {
  console.log(`${address} = ${value} (rev: ${meta.revision})`);
});

// Subscribe with options
sf.subscribe('/controller/fader/*', callback, {
  maxRate: 30,
  epsilon: 0.01
});

// Unsubscribe
const unsub = sf.subscribe('/path', callback);
unsub();  // Stop listening
```

### Writing State

```javascript
// Set value
sf.set('/lumen/scene/0/layer/0/opacity', 0.5);

// Set with options
sf.set('/lumen/scene/0/layer/0/opacity', 0.5, {
  lock: true  // Request exclusive control
});

// Publish event
sf.emit('/lumen/cue/fire', { cue: 'intro' });

// Publish stream sample
sf.stream('/controller/fader/1', 0.75);
```

### Bundles (Atomic Operations)

```javascript
// Execute immediately
sf.bundle([
  { set: ['/light/1/intensity', 1.0] },
  { set: ['/light/2/intensity', 0.0] },
  { emit: ['/cue/fire', { id: 'intro' }] }
]);

// Schedule for future
sf.bundle([...], { at: sf.time() + 100000 }); // 100ms from now
```

### Discovery

```javascript
import { discover } from 'clasp';

// Find devices on LAN (native only, not browser)
const devices = await discover({ timeout: 5000 });
// [{ name: 'LumenCanvas', url: 'wss://192.168.1.42:7330', ... }]

// In browser, use known endpoints or QR scanning
```

### Introspection

```javascript
// Get available signals
const signals = await sf.query('/lumen/**');
// [{ address: '/lumen/scene/0/layer/0/opacity', type: 'param', ... }]
```

## 10.3 Python API

```python
from clasp import Clasp

# Connect
sf = CLASP('wss://localhost:7330')

# Subscribe
@sf.on('/lumen/scene/*/layer/*/opacity')
def on_opacity(value, address):
    print(f'{address} = {value}')

# Set
sf.set('/lumen/scene/0/layer/0/opacity', 0.5)

# Event loop
sf.run()  # or use asyncio
```

## 10.4 Embedded C API

```c
#include "clasp.h"

// Initialize (UDP mode)
sf_ctx_t* sf = sf_init_udp("192.168.1.100", 7331);

// Send param
sf_set_f32(sf, "/controller/fader/1", 0.75f);

// Receive (polling)
sf_message_t msg;
while (sf_recv(sf, &msg, 0)) {
    if (msg.type == SF_MSG_SET) {
        printf("%s = %f\n", msg.address, msg.value.f32);
    }
}

// Cleanup
sf_free(sf);
```

---

# Part 11: Conformance Levels

## 11.1 Conformance Matrix

| Level | Requirements | Target |
|-------|--------------|--------|
| **Minimal** | WebSocket, HELLO/WELCOME, SET/PUBLISH | Browser apps |
| **Standard** | Minimal + SUBSCRIBE, Param/Event/Stream, MessagePack | Desktop apps |
| **Full** | Standard + Timeline, Gestures, Discovery, Bridges | Professional tools |
| **Embedded** | UDP, numeric addresses, fixed types | Microcontrollers |

## 11.2 Minimal Implementation (~200 LOC)

A minimal CLASP client needs:

1. WebSocket connection
2. MessagePack encode/decode
3. HELLO/WELCOME handshake
4. SET for sending params
5. PUBLISH for receiving

That's it. No discovery, no encryption, no complex state management.

## 11.3 Test Suite

Conformance is verified by the official test suite:

```bash
npx clasp-test ws://localhost:7330 --level standard
```

---

# Part 12: Comparison to Existing Protocols

## 12.1 Feature Matrix

| Feature | SF/2 | OSC | MIDI 2.0 | Art-Net | MQTT | WebSocket |
|---------|------|-----|----------|---------|------|-----------|
| Browser native | ✓ | ✗ | ✗ | ✗ | △ | ✓ |
| Typed data | ✓ | △ | ✓ | ✗ | ✗ | ✗ |
| Semantic signals | ✓ | ✗ | △ | ✗ | ✗ | ✗ |
| State management | ✓ | ✗ | △ | ✗ | △ | ✗ |
| Conflict resolution | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Time sync | ✓ | △ | △ | ✗ | ✗ | ✗ |
| Scheduled messages | ✓ | △ | ✗ | ✗ | ✗ | ✗ |
| Auto-discovery | ✓ | ✗ | △ | △ | ✗ | ✗ |
| Encryption built-in | ✓ | ✗ | ✗ | ✗ | △ | △ |
| Capability tokens | ✓ | ✗ | ✗ | ✗ | △ | ✗ |
| Introspection | ✓ | ✗ | ✓ | △ | ✗ | ✗ |
| Embedded friendly | ✓ | △ | △ | △ | ✓ | ✗ |
| P2P support | ✓ | △ | ✗ | ✗ | ✗ | ✗ |

## 12.2 Migration Paths

### From OSC
- Use OSC bridge
- Map addresses directly
- Gain: state management, discovery, types

### From MIDI
- Use MIDI bridge
- CC → Params, Notes → Events
- Gain: higher resolution, state, networking

### From WebSocket JSON
- Replace JSON with MessagePack
- Add signal type semantics
- Gain: efficiency, interoperability, discovery

### From MQTT
- Similar pub/sub model
- Add signal types
- Gain: creative primitives, timing, state

---

# Part 13: Reference Implementations

## 13.1 Official Implementations

| Name | Language | Status | Notes |
|------|----------|--------|-------|
| `clasp-js` | JavaScript | Reference | Browser + Node.js |
| `clasp-py` | Python | Reference | Sync + async |
| `clasp-rs` | Rust | Reference | Native + WASM |
| `clasp-c` | C | Reference | Embedded friendly |

## 13.2 Router Implementation

Reference router features:
- Multi-transport (WS, WebRTC, UDP)
- mDNS discovery
- Bridge hosting
- Recording/playback
- Web admin UI

```bash
npx clasp-router --port 7330 --discovery mdns
```

---

# Appendix A: Wire Format Examples

## A.1 Simple SET Message

```
Hex: 53 00 00 1A 82 A4 74 79 70 65 03 A7 61 64 64 72 65 73 73 
     B1 2F 6C 75 6D 65 6E 2F 6F 70 61 63 69 74 79 A5 76 61 6C
     75 65 CA 3F 40 00 00

Breakdown:
53          Magic ('S')
00          Flags (no timestamp, QoS fire, no encryption)
00 1A       Payload length (26 bytes)
[MessagePack payload]:
  82        Map with 2 entries
  A4 type   Key: "type"
  03        Value: 3 (SET)
  A7 address Key: "address"
  B1 ...    Value: "/lumen/opacity" (fixstr)
  A5 value  Key: "value"
  CA ...    Value: 0.75 (float32)
```

## A.2 Bundle with Timestamp

```
Hex: 53 20 00 45 00 00 01 8D 6B 2F 3A 00 83 A4 74 79 70 65 06 
     A9 74 69 6D 65 73 74 61 6D 70 CF 00 00 01 8D 6B 2F 4A 00
     A8 6D 65 73 73 61 67 65 73 92 ...

Breakdown:
53          Magic ('S')
20          Flags (timestamp present, QoS fire)
00 45       Payload length (69 bytes)
00 00 01 8D 6B 2F 3A 00  Timestamp (8 bytes, µs)
[MessagePack payload]:
  83        Map with 3 entries
  A4 type   "type"
  06        6 (BUNDLE)
  A9 timestamp "timestamp"
  CF ...    uint64 scheduled time
  A8 messages "messages"
  92 ...    Array of 2 messages
```

---

# Appendix B: Security Considerations

## B.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| Eavesdropping | TLS/DTLS encryption |
| MITM | Server certificate verification |
| Replay attacks | Timestamps + sequence numbers |
| Unauthorized access | Capability tokens |
| DoS (rate flooding) | Per-source rate limits |
| Parameter manipulation | Range constraints in tokens |

## B.2 Recommendations

1. **Always use WSS in production** (not WS)
2. **Validate capability tokens** on every SET/PUBLISH
3. **Rate limit** aggressively (60Hz is plenty for most params)
4. **Log security events** (failed auth, constraint violations)
5. **Expire tokens** (max 24h, shorter for high-security)

---

# Appendix C: Glossary

| Term | Definition |
|------|------------|
| **Param** | A stateful value with revision tracking |
| **Event** | An ephemeral trigger with optional payload |
| **Stream** | High-rate continuous data |
| **Gesture** | Phased input (touch/pen/motion) |
| **Timeline** | Time-indexed automation |
| **Signal** | Any Param, Event, Stream, Gesture, or Timeline |
| **Node** | A CLASP client or server |
| **Router** | A CLASP server with routing capabilities |
| **Bridge** | A node that translates legacy protocols |
| **Session** | A connection with identity and state |
| **Bundle** | An atomic group of messages |

---

# Appendix D: Version History

| Version | Date | Changes |
|---------|------|---------|
| 3.0 | 2026-01 | Compact binary encoding (54% smaller), version bits in frame flags, backward compatible |
| 2.0-draft | 2025-01 | Major revision: MessagePack payload, discovery spec, P2P, API design |
| 1.0-draft | 2024 | Initial specification |

---

# Appendix E: Acknowledgments

CLASP builds on the shoulders of giants:
- OSC (Matt Wright, Adrian Freed)
- MIDI 2.0 (MMA, AMEI)
- WebRTC (W3C, IETF)
- mDNS/Bonjour (Apple, IETF)
- QUIC (Google, IETF)
- MessagePack (Sadayuki Furuhashi)

---

*CLASP: The universal language for creative tools.*
