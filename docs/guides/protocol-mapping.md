# Protocol Mapping Examples

This document shows exactly how messages translate between external protocols and CLASP, in both directions.

## Understanding the Mapping

Each bridge translates messages bidirectionally:

```
External Protocol → Bridge → CLASP Format → Bridge → External Protocol
```

All bridges add a namespace prefix to addresses (e.g., `/osc`, `/midi`, `/dmx`).

---

## OSC ↔ CLASP

### OSC → CLASP

**OSC Message:**
```
Address: /1/fader1
Type: float32
Value: 0.75
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/osc/1/fader1",
  "value": 0.75,
  "revision": 42
}
```

**Binary CLASP Frame:**
```
53 01 00 1F 21 07 00 0E 2F 6F 73 63 2F 31 2F 66 61 64 65 72 31 3F E8 00 00 00 00 00 00
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Value: 0.75 (f64)
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Address: "/osc/1/fader1"
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Address length: 14
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Value type: f64 (0x07)
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Message type: SET (0x21)
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Payload length: 31 bytes
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Flags
│  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  │  └─ Magic: 'S' (0x53)
```

### CLASP → OSC

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/osc/cue/fire",
  "value": {"cue": "intro", "transition": "fade"}
}
```

**OSC Message:**
```
Address: /cue/fire
Types: s, s
Values: "intro", "fade"
```

**OSC Bundle (with timetag):**
```
#bundle [timetag]
  /cue/fire
    ,ss
    intro
    fade
```

---

## MIDI ↔ CLASP

### MIDI → CLASP

**MIDI CC Message:**
```
Channel: 1
CC Number: 7 (Volume)
Value: 100 (0-127)
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/midi/ch1/cc/7",
  "value": 100,
  "meta": {
    "unit": "midi",
    "range": [0, 127]
  }
}
```

**MIDI Note On:**
```
Channel: 1
Note: 60 (C4)
Velocity: 100
```

**CLASP Event:**
```json
{
  "type": "PUBLISH",
  "address": "/midi/ch1/note/60",
  "payload": {
    "velocity": 100,
    "state": "on"
  }
}
```

### CLASP → MIDI

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/midi/ch1/cc/7",
  "value": 75
}
```

**MIDI CC:**
```
Channel: 1
CC: 7
Value: 75
```

**CLASP Event:**
```json
{
  "type": "PUBLISH",
  "address": "/midi/ch1/note/60",
  "payload": {"velocity": 0}
}
```

**MIDI Note Off:**
```
Channel: 1
Note: 60
Velocity: 0
```

---

## DMX ↔ CLASP

### DMX → CLASP

**DMX Universe 1, Channel 47:**
```
Universe: 1
Channel: 47
Value: 255 (0-255)
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/dmx/1/47",
  "value": 255,
  "meta": {
    "unit": "dmx",
    "range": [0, 255]
  }
}
```

**DMX Multiple Channels (RGB fixture):**
```
Universe: 1
Channels: 1=255 (R), 2=128 (G), 3=64 (B)
```

**CLASP Messages (3 separate SETs or 1 BUNDLE):**
```json
{
  "type": "BUNDLE",
  "messages": [
    {"type": "SET", "address": "/dmx/1/1", "value": 255},
    {"type": "SET", "address": "/dmx/1/2", "value": 128},
    {"type": "SET", "address": "/dmx/1/3", "value": 64}
  ]
}
```

### CLASP → DMX

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/dmx/1/47",
  "value": 200
}
```

**DMX Output:**
```
Universe: 1
Channel: 47
Value: 200
```

**CLASP BUNDLE:**
```json
{
  "type": "BUNDLE",
  "messages": [
    {"type": "SET", "address": "/dmx/1/1", "value": 255},
    {"type": "SET", "address": "/dmx/1/2", "value": 128},
    {"type": "SET", "address": "/dmx/1/3", "value": 64}
  ]
}
```

**DMX Output (atomic):**
```
Universe: 1
Channels: 1=255, 2=128, 3=64 (sent together)
```

---

## Art-Net ↔ CLASP

### Art-Net → CLASP

**Art-Net Packet:**
```
Net: 0
Subnet: 1
Universe: 2
Channel: 47
Value: 255
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/artnet/0/1/2/47",
  "value": 255,
  "meta": {
    "net": 0,
    "subnet": 1,
    "universe": 2
  }
}
```

**Art-Net Full Universe (512 channels):**
```
Net: 0, Subnet: 0, Universe: 1
512 channels of data
```

**CLASP:** Individual SET messages or BUNDLE with 512 messages

### CLASP → Art-Net

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/artnet/0/1/2/47",
  "value": 200
}
```

**Art-Net Packet:**
```
Net: 0
Subnet: 1
Universe: 2
Channel: 47
Value: 200
```

---

## MQTT ↔ CLASP

### MQTT → CLASP

**MQTT Message:**
```
Topic: sensors/temperature/room1
Payload: "23.5"
QoS: 1
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/mqtt/sensors/temperature/room1",
  "value": 23.5
}
```

**MQTT JSON Payload:**
```
Topic: home/living/light
Payload: {"brightness": 75, "color": "warm"}
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/mqtt/home/living/light",
  "value": {
    "brightness": 75,
    "color": "warm"
  }
}
```

### CLASP → MQTT

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/mqtt/home/living/light/brightness",
  "value": 80
}
```

**MQTT Message:**
```
Topic: home/living/light/brightness
Payload: "80"
QoS: 1
```

**CLASP BUNDLE:**
```json
{
  "type": "BUNDLE",
  "messages": [
    {"type": "SET", "address": "/mqtt/sensors/temp", "value": 23.5},
    {"type": "SET", "address": "/mqtt/sensors/humidity", "value": 45.2}
  ]
}
```

**MQTT:** Two separate MQTT messages (one per topic)

---

## WebSocket ↔ CLASP

### WebSocket (JSON) → CLASP

**WebSocket Message:**
```json
{
  "type": "set",
  "path": "/lights/brightness",
  "value": 0.75
}
```

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/websocket/lights/brightness",
  "value": 0.75
}
```

### CLASP → WebSocket (JSON)

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/websocket/lights/brightness",
  "value": 0.8
}
```

**WebSocket Message:**
```json
{
  "type": "set",
  "path": "/lights/brightness",
  "value": 0.8
}
```

**Note:** Native CLASP WebSocket uses binary encoding, not JSON. This mapping is for generic WebSocket bridges.

---

## HTTP ↔ CLASP

### HTTP → CLASP

**HTTP Request:**
```
GET /api/lights/brightness
```

**CLASP GET:**
```json
{
  "type": "GET",
  "address": "/http/api/lights/brightness"
}
```

**HTTP Response (from CLASP SNAPSHOT):**
```json
{
  "type": "SNAPSHOT",
  "params": [
    {
      "address": "/http/api/lights/brightness",
      "value": 0.75,
      "revision": 42
    }
  ]
}
```

**HTTP Response:**
```json
{
  "brightness": 0.75
}
```

### CLASP → HTTP

**CLASP Message:**
```json
{
  "type": "SET",
  "address": "/http/api/lights/brightness",
  "value": 0.8
}
```

**HTTP Request:**
```
PUT /api/lights/brightness
Content-Type: application/json

{"brightness": 0.8}
```

---

## Complex Examples

### Multi-Protocol Chain

**Scenario:** TouchOSC controls DMX lights via CLASP

1. **TouchOSC sends OSC:**
   ```
   /1/fader1 ,f 0.75
   ```

2. **OSC Bridge converts to CLASP:**
   ```json
   {
     "type": "SET",
     "address": "/osc/1/fader1",
     "value": 0.75
   }
   ```

3. **CLASP Router routes to DMX Bridge**

4. **DMX Bridge converts to DMX:**
   ```
   Universe: 1
   Channel: 1
   Value: 191 (0.75 × 255)
   ```

### State Synchronization

**Scenario:** Multiple clients control the same parameter

1. **Client A (OSC) sets:**
   ```
   OSC: /1/fader1 ,f 0.5
   → CLASP: SET /osc/1/fader1 0.5 (revision: 10)
   ```

2. **Client B (MQTT) sets:**
   ```
   MQTT: sensors/fader1 = 0.8
   → CLASP: SET /mqtt/sensors/fader1 0.8 (revision: 11)
   ```

3. **CLASP Router applies last-write-wins:**
   - Current value: 0.8 (revision: 11)
   - All subscribers receive update

4. **OSC Bridge sends back to TouchOSC:**
   ```
   OSC: /1/fader1 ,f 0.8
   ```

### Bundle Translation

**CLASP BUNDLE:**
```json
{
  "type": "BUNDLE",
  "timestamp": 1704067200000,
  "messages": [
    {"type": "SET", "address": "/dmx/1/1", "value": 255},
    {"type": "SET", "address": "/dmx/1/2", "value": 128},
    {"type": "PUBLISH", "address": "/osc/cue/fire", "payload": {"id": "intro"}}
  ]
}
```

**OSC Bundle:**
```
#bundle [1704067200000]
  /cue/fire
    ,s
    intro
```

**DMX:** Three channel updates sent atomically

---

## Type Conversion Reference

| External Protocol | CLASP Type | Notes |
|-------------------|------------|-------|
| OSC int32 | `Int` | Direct mapping |
| OSC float32 | `Float` | Direct mapping |
| OSC string | `String` | Direct mapping |
| OSC blob | `Bytes` | Direct mapping |
| MIDI CC (0-127) | `Int` | Range: 0-127 |
| MIDI Note | `Event` | With velocity payload |
| DMX (0-255) | `Int` | Range: 0-255 |
| MQTT number | `Int` or `Float` | Auto-detected |
| MQTT JSON | `Map` | Parsed object |
| HTTP JSON | `Map` | Parsed object |

---

## Best Practices

1. **Use BUNDLEs for atomic operations** - Multiple DMX channels should update together
2. **Respect protocol ranges** - DMX is 0-255, MIDI CC is 0-127
3. **Handle type conversion** - Some protocols are integer-only
4. **Consider namespace prefixes** - All bridges add prefixes to avoid conflicts
5. **Use appropriate signal types** - Events for triggers, Params for state

---

## Next Steps

- See [Bridge Setup Guide](./bridge-setup.md) for configuration
- Check [Protocol-Specific Docs](../protocols/README.md) for advanced features
- Review [Integration Examples](../integrations/README.md) for real-world patterns
