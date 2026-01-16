# OSC Bridge

The OSC (Open Sound Control) bridge provides bidirectional communication with OSC-compatible applications like TouchOSC, Max/MSP, Ableton Live, Resolume, and many others.

## Configuration

```rust
use clasp_bridge::{OscBridge, OscBridgeConfig};

let config = OscBridgeConfig {
    bind_addr: "0.0.0.0:9000".to_string(),  // Listen address
    remote_addr: Some("192.168.1.100:8000".to_string()), // Send address
    namespace: "/osc".to_string(),           // CLASP namespace prefix
};

let bridge = OscBridge::new(config);
```

### CLI Usage

```bash
# Listen on port 9000
clasp osc --port 9000

# Listen on specific interface
clasp osc --bind 192.168.1.50 --port 9000
```

### Desktop App

1. Click **ADD** in the sidebar
2. Select **OSC Server**
3. Configure bind address and port
4. Click **START SERVER**

## Address Mapping

OSC addresses are mapped to CLASP addresses with the namespace prefix:

| OSC Address | CLASP Address |
|-------------|---------------|
| `/1/fader1` | `/osc/1/fader1` |
| `/track/1/volume` | `/osc/track/1/volume` |
| `/cue/go` | `/osc/cue/go` |

## Value Type Conversion

| OSC Type | CLASP Type |
|----------|------------|
| `i` (int32) | `Int` |
| `f` (float32) | `Float` |
| `s` (string) | `String` |
| `b` (blob) | `Bytes` |
| `T` (true) | `Bool(true)` |
| `F` (false) | `Bool(false)` |
| `N` (nil) | `Null` |
| Arrays | `Array` |

## Examples

### Receiving OSC Messages

```rust
use clasp_bridge::{OscBridge, OscBridgeConfig, Bridge, BridgeEvent};
use clasp_core::Message;

#[tokio::main]
async fn main() {
    let config = OscBridgeConfig {
        bind_addr: "0.0.0.0:9000".to_string(),
        ..Default::default()
    };

    let mut bridge = OscBridge::new(config);
    let mut events = bridge.start().await.unwrap();

    while let Some(event) = events.recv().await {
        if let BridgeEvent::ToClasp(Message::Set(msg)) = event {
            println!("Address: {}", msg.address);
            println!("Value: {:?}", msg.value);
        }
    }
}
```

### Sending OSC Messages

```rust
use clasp_bridge::{OscBridge, OscBridgeConfig, Bridge};
use clasp_core::{Message, SetMessage, Value};

async fn send_osc(bridge: &OscBridge) {
    let msg = Message::Set(SetMessage {
        address: "/osc/1/fader1".to_string(),
        value: Value::Float(0.75),
        revision: None,
        lock: false,
        unlock: false,
    });

    bridge.send(msg).await.unwrap();
}
```

## Common Applications

| Application | Default Port | Notes |
|-------------|--------------|-------|
| TouchOSC | 9000 | Mobile control surfaces |
| Resolume | 7000 | VJ software |
| Max/MSP | 8000 | Visual programming |
| Ableton Live | 9001 | DAW (via Max4Live) |
| QLab | 53000 | Show control |
