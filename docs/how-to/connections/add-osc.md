# Add OSC

Connect OSC (Open Sound Control) devices and applications to CLASP.

## Prerequisites

- Running CLASP router
- OSC-capable device or application (TouchOSC, Resolume, etc.)

## Two Integration Modes

CLASP supports OSC in two ways:

1. **OSC Server Adapter** (new in 3.1.0): The router accepts OSC messages directly via UDP
2. **OSC Bridge**: A separate process that translates between OSC and CLASP

## OSC Server Adapter (Recommended)

The OSC server adapter lets the router accept OSC messages directly on a UDP port. No separate bridge process needed.

```bash
# Start router with OSC support
clasp server --osc-port 8000

# Custom namespace
clasp server --osc-port 8000 --osc-namespace /touchosc
```

OSC sources are tracked as sessions. Sessions expire after 30 seconds of inactivity by default.

### Address Mapping

| OSC Address | CLASP Address |
|-------------|---------------|
| `/synth/volume` | `/osc/synth/volume` |
| `/fader/1` | `/osc/fader/1` |

---

## OSC Bridge

### CLI

```bash
clasp osc --port 9000
```

This creates a bridge that:
- Listens for OSC on UDP port 9000
- Connects to CLASP router on `localhost:7330`
- Translates OSC ↔ CLASP bidirectionally

### Desktop App

1. Click **Add Protocol**
2. Select **OSC (Open Sound Control)**
3. Set port (default: 9000)
4. Click **Start**

## Address Mapping

OSC addresses are prefixed with `/osc/`:

```
OSC: /synth/osc1/cutoff
CLASP: /osc/synth/osc1/cutoff

OSC: /fader/1
CLASP: /osc/fader/1
```

## Send to OSC Devices

From CLASP, set values that will be sent as OSC:

```javascript
// This sends OSC message to devices
await client.set('/osc/synth/osc1/cutoff', 0.75);
```

## Receive from OSC Devices

Subscribe to OSC messages from devices:

```javascript
client.on('/osc/**', (value, address) => {
  console.log(`OSC: ${address} = ${value}`);
});
```

## OSC Bundles

OSC bundles map to CLASP bundles:

```javascript
// Send multiple OSC messages atomically
await client.bundle([
  { set: ['/osc/light/1/dim', 1.0] },
  { set: ['/osc/light/2/dim', 0.0] }
]);
```

## Type Mapping

| OSC Type | CLASP Type |
|----------|------------|
| int32 | Int |
| float32 | Float |
| string | String |
| blob | Bytes |
| True/False | Bool |
| Nil | Null |
| timetag | Timestamp |

## Custom Prefix

Use a custom prefix instead of `/osc/`:

```bash
clasp osc --port 9000 --prefix /touchosc
```

```javascript
// Now accessible at /touchosc/...
client.on('/touchosc/**', callback);
```

## Multiple OSC Sources

Run multiple bridges with different prefixes:

```bash
# TouchOSC on 9000
clasp osc --port 9000 --prefix /touchosc

# Resolume on 9001
clasp osc --port 9001 --prefix /resolume
```

## Sending to Specific Hosts

By default, the bridge sends OSC back to the source address. To send to a specific host:

```bash
clasp osc --port 9000 --send-to 192.168.1.100:8000
```

## Troubleshooting

### OSC not received

1. Check port isn't blocked:
   ```bash
   lsof -i :9000
   ```

2. Verify OSC source is sending to correct port

3. Test with OSC monitor app

### Wrong values

Check type conversion. OSC uses float32, CLASP uses float64.

## Example: TouchOSC Setup

1. Start bridge: `clasp osc --port 9000`
2. In TouchOSC settings, set OSC host to your computer's IP
3. Set OSC send port to 9000
4. Set OSC receive port to 9000 (for feedback)

```javascript
// Receive from TouchOSC
client.on('/osc/**', (value, address) => {
  console.log(address, value);
});

// Send to TouchOSC (feedback)
await client.set('/osc/fader/1', 0.75);
```

## Next Steps

- [TouchOSC Integration](../../integrations/touchosc.md)
- [OSC Bridge Reference](../../reference/bridges/osc.md)
