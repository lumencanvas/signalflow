# Protocol Connections

CLASP supports connecting multiple protocols to the central router. Each protocol connection translates bidirectionally between its native protocol and CLASP.

## Supported Protocols

| Protocol | Direction | Transport | Use Cases |
|----------|-----------|-----------|-----------|
| [OSC](osc.md) | Bidirectional | UDP | Audio software, VJ apps, TouchOSC |
| [MIDI](midi.md) | Bidirectional | USB/Virtual | DAWs, controllers, synthesizers |
| [Art-Net](artnet.md) | Bidirectional | UDP | DMX lighting over Ethernet |
| [DMX](dmx.md) | Output | Serial | Direct DMX via USB interfaces |
| [MQTT](mqtt.md) | Bidirectional | TCP/TLS | IoT devices, home automation |
| [WebSocket](websocket.md) | Bidirectional | TCP | Web apps, real-time UIs |
| [Socket.IO](socketio.md) | Bidirectional | TCP | Node.js apps, chat systems |
| [HTTP](http.md) | Bidirectional | TCP | REST APIs, webhooks |

## Protocol Connection Architecture

Each protocol connection implements the `Bridge` trait:

```rust
#[async_trait]
pub trait Bridge: Send + Sync {
    fn config(&self) -> &BridgeConfig;
    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>>;
    async fn stop(&mut self) -> Result<()>;
    async fn send(&self, message: Message) -> Result<()>;
    fn is_running(&self) -> bool;
    fn namespace(&self) -> &str;
}
```

### BridgeEvent

Protocol connections emit events through a channel:

```rust
pub enum BridgeEvent {
    ToClasp(Message),  // Message received from external protocol
    Connected,              // Connection established successfully
    Disconnected { reason: Option<String> },
    Error(String),
}
```

## Common Configuration

All protocol connections share some common configuration:

```rust
pub struct BridgeConfig {
    pub name: String,        // Human-readable name
    pub protocol: String,    // Protocol identifier
    pub bidirectional: bool, // Whether bridge can send and receive
    // ...
}
```

## Namespace Mapping

Each protocol connection has a namespace that prefixes all addresses:

| Protocol | Default Namespace | Example Address |
|----------|-------------------|-----------------|
| OSC | `/osc` | `/osc/1/fader1` |
| MIDI | `/midi` | `/midi/ch1/note/60` |
| MQTT | `/mqtt` | `/mqtt/sensors/temp` |
| HTTP | `/http` | `/http/api/status` |

## Next Steps

- [OSC Connection](osc.md) - Open Sound Control
- [MIDI Connection](midi.md) - Musical Instrument Digital Interface
- [MQTT Connection](mqtt.md) - IoT messaging
- [WebSocket Connection](websocket.md) - Real-time web
