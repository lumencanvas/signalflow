# CLASP Quick Reference

## Connection
```javascript
const clasp = new Clasp('wss://localhost:7330');
await clasp.connect();
```

## Reading
```javascript
// Get current value
const value = await clasp.get('/lumen/layer/0/opacity');

// Subscribe to changes
clasp.on('/lumen/layer/*/opacity', (value, address) => {
  console.log(`${address} = ${value}`);
});

// With options
clasp.on('/controller/*', callback, { maxRate: 30, epsilon: 0.01 });
```

## Writing
```javascript
// Set param (stateful)
clasp.set('/lumen/layer/0/opacity', 0.75);

// Emit event (ephemeral)
clasp.emit('/cue/fire', { id: 'intro' });

// Stream (high-rate)
clasp.stream('/fader/1', 0.5);
```

## Bundles (Atomic)
```javascript
clasp.bundle([
  { set: ['/light/1', 1.0] },
  { set: ['/light/2', 0.0] }
]);

// Scheduled
clasp.bundle([...], { at: clasp.time() + 100000 });  // 100ms later
```

## Signal Types

| Type | Use | QoS | State? |
|------|-----|-----|--------|
| Param | Values with history | Confirm | Y |
| Event | Triggers | Confirm | N |
| Stream | High-rate data | Fire | N |
| Gesture | Touch/motion | Fire | Phase |
| Timeline | Automation | Commit | Y |

## Discovery
```javascript
// mDNS service: _clasp._tcp.local
// UDP broadcast: port 7331

// In browser (no native discovery):
const clasp = new Clasp('wss://192.168.1.42:7330');
```

## Frame Format (4 bytes minimum)
```
[0]    Magic 'C' (0x43)
[1]    Flags (QoS, timestamp, encrypted, compressed)
[2-3]  Payload length (uint16 BE)
[4+]   MessagePack payload
```

## Bridge Mappings

### MIDI -> CLASP
```
Note On/Off  -> /midi/{dev}/note     Event
CC           -> /midi/{dev}/cc/{n}   Param u8
Pitch Bend   -> /midi/{dev}/bend     Param i16
```

### OSC -> CLASP
```
/synth/cutoff ,f 0.5  ->  SET /osc/synth/cutoff 0.5
```

### DMX -> CLASP
```
Universe 1, Ch 47  ->  /dmx/1/47  Param u8
```

## Addresses
```
/namespace/category/instance/property
/lumen/scene/0/layer/3/opacity
/midi/launchpad/cc/74

Wildcards (subscribe only):
*   = one segment
**  = any segments

/lumen/scene/*/layer/**/opacity
```

## Common Ports
- WebSocket: 7330
- UDP Discovery: 7331
- mDNS: 5353 (standard)

## Error Codes
- 1xx: Protocol errors
- 2xx: Address errors
- 3xx: Permission errors
- 4xx: State errors
- 5xx: Server errors

## Security
```javascript
// Capability token (JWT)
{
  "clasp": {
    "read": ["/lumen/**"],
    "write": ["/lumen/layer/*/opacity"],
    "constraints": {
      "/lumen/layer/*/opacity": { "range": [0, 1], "maxRate": 60 }
    }
  }
}
```

## Timing
```javascript
// Sync happens automatically on connect
// All timestamps: microseconds since session start

// Schedule for future
clasp.bundle([...], { at: clasp.time() + 500000 });  // 500ms
```
