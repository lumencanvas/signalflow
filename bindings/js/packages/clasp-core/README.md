# @clasp-to/core

JavaScript/TypeScript client for CLASP - Creative Low-Latency Application Streaming Protocol.

[![npm](https://img.shields.io/npm/v/@clasp-to/core)](https://www.npmjs.com/package/@clasp-to/core)
[![License](https://img.shields.io/npm/l/@clasp-to/core)](LICENSE)

## Installation

```bash
npm install @clasp-to/core
```

## Quick Start

```typescript
import { Clasp, ClaspBuilder } from '@clasp-to/core';

// Connect to a CLASP server
const client = await new ClaspBuilder('ws://localhost:7330')
  .withName('My App')
  .connect();

// Subscribe to parameter changes
client.on('/lumen/layer/*/opacity', (value, address) => {
  console.log(`${address} = ${value}`);
});

// Set a parameter
await client.set('/lumen/layer/0/opacity', 0.75);

// Get a parameter
const opacity = await client.get('/lumen/layer/0/opacity');

// Emit an event
await client.emit('/cue/fire', { id: 'intro' });

// Stream high-rate data
client.stream('/fader/1', 0.5);

// Close when done
await client.close();
```

## API

### ClaspBuilder

```typescript
const client = await new ClaspBuilder(url)
  .withName('Client Name')        // Set client name
  .withFeatures(['param', 'event']) // Specify features
  .withReconnect(true, 5000)      // Auto-reconnect with interval
  .connect();
```

### Clasp Client

#### Reading

- `get(address)` - Get parameter value
- `on(pattern, callback)` - Subscribe to address pattern
- `cached(address)` - Get cached value (sync)

#### Writing

- `set(address, value)` - Set parameter (stateful)
- `emit(address, payload?)` - Emit event (ephemeral)
- `stream(address, value)` - Stream sample (high-rate)

#### Bundles

```typescript
// Atomic bundle
client.bundle([
  { set: ['/light/1', 1.0] },
  { set: ['/light/2', 0.0] }
]);

// Scheduled bundle
client.bundle([...], { at: client.time() + 100000 }); // 100ms later
```

#### Utilities

- `time()` - Get server-synced time (microseconds)
- `connected` - Check connection status
- `sessionId` - Get session ID
- `close()` - Close connection

### Address Patterns

CLASP supports wildcards in subscriptions:

| Pattern | Matches |
|---------|---------|
| `/lights/front` | Exact match |
| `/lights/*` | Single segment wildcard |
| `/lights/**` | Multi-segment wildcard |

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

MIT

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
