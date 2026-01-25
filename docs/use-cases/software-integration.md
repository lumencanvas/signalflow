# Software Integration

Connect multiple applications together using CLASP as a universal bridge.

## Overview

CLASP enables real-time communication between different software applications:

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   App A     │  │   App B     │  │   App C     │  │   App D     │
│   (OSC)     │  │   (MIDI)    │  │  (WebSocket)│  │   (HTTP)    │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │                │
       └────────────────┴────────────────┴────────────────┘
                                │
                         ┌──────▼──────┐
                         │    CLASP    │
                         │   Router    │
                         └─────────────┘
```

## Common Patterns

### Protocol Translation

Bridge applications using different protocols:

```javascript
// OSC application sends data
// → CLASP receives at /osc/app/value

// Forward to MIDI application
client.on('/osc/app/value', async (value) => {
  // Scale OSC float (0-1) to MIDI CC (0-127)
  const midiValue = Math.round(value * 127);
  await client.set('/midi/out/cc/1/1', midiValue);
});

// Forward to HTTP endpoint
client.on('/osc/app/value', async (value) => {
  await client.set('/http/api/value', { value });
});
```

### Data Normalization

Normalize values between different systems:

```javascript
// Different sources send values in different ranges
const normalizers = {
  '/osc/fader': (v) => v,                    // Already 0-1
  '/midi/cc/7': (v) => v / 127,              // MIDI 0-127 → 0-1
  '/http/slider': (v) => v / 100,            // Percent 0-100 → 0-1
  '/mqtt/sensor': (v) => (v - 20) / 80       // Celsius 20-100 → 0-1
};

// Create unified control signal
for (const [source, normalize] of Object.entries(normalizers)) {
  client.on(source, async (value) => {
    const normalized = normalize(value);
    await client.set('/control/unified', normalized);
  });
}

// Any subscriber gets consistent 0-1 values
client.on('/control/unified', (value) => {
  console.log(`Unified control: ${value}`); // Always 0-1
});
```

### Event Routing

Route events between applications:

```javascript
// Define event routes
const routes = {
  '/app/a/button/1': ['/app/b/trigger', '/app/c/cue/1'],
  '/app/a/button/2': ['/app/b/stop'],
  '/midi/note/60': ['/app/a/action', '/app/b/action', '/app/c/action']
};

// Set up routing
for (const [source, destinations] of Object.entries(routes)) {
  client.on(source, async (value) => {
    const ops = destinations.map(dest => ({ emit: [dest, value] }));
    await client.bundle(ops);
  });
}
```

### State Synchronization

Keep state synchronized across applications:

```javascript
// Master state
const state = {
  volume: 0.5,
  muted: false,
  mode: 'normal'
};

// Update from any source
client.on('/control/volume', (value) => {
  state.volume = value;
  broadcastState();
});

client.on('/control/muted', (value) => {
  state.muted = value;
  broadcastState();
});

// Broadcast to all connected apps
async function broadcastState() {
  await client.bundle([
    { set: ['/osc/app/volume', state.volume] },
    { set: ['/midi/out/cc/1/7', Math.round(state.volume * 127)] },
    { set: ['/mqtt/state/volume', state.volume] },
    { set: ['/http/broadcast/state', state] }
  ]);
}

// New apps can request current state
client.on('/state/request', async () => {
  await client.set('/state/current', state);
});
```

## Integration Scenarios

### DAW + Visual Software

Connect Ableton Live to Resolume:

```javascript
// Ableton sends tempo via OSC
client.on('/osc/live/tempo', async (bpm) => {
  await client.set('/osc/resolume/composition/bpm', bpm);
});

// Ableton sends transport state
client.on('/osc/live/playing', async (playing) => {
  if (playing) {
    await client.emit('/osc/resolume/composition/play', 1);
  } else {
    await client.emit('/osc/resolume/composition/pause', 1);
  }
});

// Ableton sends track levels
client.on('/osc/live/track/*/volume', async (value, address) => {
  const track = address.split('/')[4];
  // Map track volumes to visual layer opacities
  await client.set(`/osc/resolume/layers/${track}/video/opacity`, value);
});
```

### Game Engine + Hardware

Connect Unity/Unreal to physical devices:

```javascript
// Game sends events
client.on('/game/explosion', async (data) => {
  // Trigger DMX strobe
  await client.set('/artnet/0/0/0/1', 255);
  setTimeout(() => client.set('/artnet/0/0/0/1', 0), 100);

  // Trigger haptic feedback
  await client.emit('/midi/out/note/60', 127);
});

// Game sends player position
client.on('/game/player/position', async (pos) => {
  // Pan lights based on player position
  const pan = Math.round((pos.x + 1) * 127); // -1 to 1 → 0 to 255
  await client.set('/artnet/0/0/0/2', pan);
});

// Physical controller sends input to game
client.on('/midi/in/cc/1/*', async (value, address) => {
  const cc = address.split('/').pop();
  await client.set(`/game/input/axis/${cc}`, value / 127);
});
```

### Web App + Desktop App

Bridge browser and desktop applications:

```javascript
// Web app (browser) connects via WebSocket
// Desktop app connects via native client

// Web app sends commands
client.on('/webapp/command/*', async (value, address) => {
  const command = address.split('/').pop();

  switch (command) {
    case 'open':
      await client.emit('/desktop/file/open', value);
      break;
    case 'save':
      await client.emit('/desktop/file/save', value);
      break;
    case 'render':
      await client.emit('/desktop/render/start', value);
      break;
  }
});

// Desktop app sends status updates
client.on('/desktop/status', async (status) => {
  await client.set('/webapp/status', status);
});

// Desktop app sends render progress
client.on('/desktop/render/progress', async (percent) => {
  await client.set('/webapp/progress', percent);
});
```

### Microservices Communication

Connect microservices:

```javascript
// Service A: Data ingestion
client.on('/ingest/data', async (data) => {
  // Validate and forward
  if (validateData(data)) {
    await client.emit('/process/queue', data);
  } else {
    await client.emit('/errors/validation', { data, error: 'Invalid format' });
  }
});

// Service B: Processing
client.on('/process/queue', async (data) => {
  const result = await processData(data);
  await client.emit('/output/ready', result);
});

// Service C: Output
client.on('/output/ready', async (result) => {
  await saveToDatabase(result);
  await client.emit('/notifications/complete', { id: result.id });
});

// Service D: Monitoring
client.on('/**', (value, address) => {
  logMetric(address, value);
});
```

## Cross-Language Communication

CLASP clients in different languages can communicate:

```javascript
// JavaScript client
const { Clasp } = require('@clasp-to/core');
const client = await Clasp.connect('ws://localhost:7330');

client.on('/python/result', (value) => {
  console.log('Python says:', value);
});

await client.set('/js/request', { compute: 'fibonacci', n: 40 });
```

```python
# Python client
import asyncio
from clasp import Clasp

async def main():
    client = await Clasp.connect('ws://localhost:7330')

    @client.on('/js/request')
    async def handle_request(value):
        if value['compute'] == 'fibonacci':
            result = fibonacci(value['n'])
            await client.set('/python/result', result)

    await client.run_forever()

asyncio.run(main())
```

```rust
// Rust client
use clasp_client::{Clasp, ClaspBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("Rust Worker")
        .connect()
        .await?;

    client.on("/js/request", |value, _addr| async move {
        // Process with Rust's performance
        let result = heavy_computation(value);
        // Note: would need client reference here in real code
        Ok(())
    }).await?;

    Ok(())
}
```

## Middleware Pattern

Insert processing between sources and destinations:

```javascript
// Logging middleware
function withLogging(handler) {
  return async (value, address) => {
    console.log(`[${new Date().toISOString()}] ${address}:`, value);
    await handler(value, address);
  };
}

// Rate limiting middleware
function withRateLimit(handler, maxRate) {
  let lastCall = 0;
  const minInterval = 1000 / maxRate;

  return async (value, address) => {
    const now = Date.now();
    if (now - lastCall >= minInterval) {
      lastCall = now;
      await handler(value, address);
    }
  };
}

// Validation middleware
function withValidation(handler, schema) {
  return async (value, address) => {
    if (validate(value, schema)) {
      await handler(value, address);
    } else {
      console.error('Validation failed:', address, value);
    }
  };
}

// Compose middleware
const processValue = withLogging(
  withRateLimit(
    withValidation(
      async (value, address) => {
        await client.set('/processed' + address, value);
      },
      { type: 'number', min: 0, max: 1 }
    ),
    30 // 30 calls/sec max
  )
);

client.on('/input/**', processValue);
```

## Error Handling

Robust error handling for integrations:

```javascript
// Dead letter queue for failed operations
client.on('/operations/*', async (value, address) => {
  try {
    await processOperation(value);
    await client.emit('/operations/success', { address, value });
  } catch (error) {
    await client.emit('/operations/failed', {
      address,
      value,
      error: error.message,
      timestamp: Date.now()
    });
  }
});

// Retry failed operations
client.on('/operations/failed', async (failure) => {
  if (failure.retryCount < 3) {
    await sleep(1000 * Math.pow(2, failure.retryCount));
    await client.emit(failure.address, {
      ...failure.value,
      retryCount: (failure.retryCount || 0) + 1
    });
  } else {
    await client.emit('/operations/deadletter', failure);
  }
});
```

## Next Steps

- [Protocol Reference](../reference/protocol/overview.md)
- [Cross-Language Tutorial](../tutorials/cross-language-chat.md)
- [Add WebSocket Bridge](../how-to/connections/add-websocket.md)
