# CLASP Python Client

A Python client for the CLASP protocol - Creative Low-Latency Application Streaming Protocol for creative tools communication.

## Installation

```bash
pip install clasp-to
```

## Quick Start

```python
import asyncio
from clasp import Clasp

async def main():
    # Connect to a CLASP server
    sf = Clasp('ws://localhost:7330')
    await sf.connect()

    # Subscribe to parameter changes
    @sf.on('/lumen/layer/*/opacity')
    def on_opacity(value, address):
        print(f'{address} = {value}')

    # Set a parameter
    await sf.set('/lumen/layer/0/opacity', 0.75)

    # Get a parameter
    opacity = await sf.get('/lumen/layer/0/opacity')
    print(f'Current opacity: {opacity}')

    # Emit an event
    await sf.emit('/lumen/cue/fire', {'cue': 1})

    # Keep running
    sf.run()

asyncio.run(main())
```

## Builder Pattern

```python
from clasp import ClaspBuilder

async def main():
    client = await (
        ClaspBuilder('ws://localhost:7330')
        .with_name('My Python App')
        .with_features(['param', 'event'])
        .with_reconnect(True, interval=5.0)
        .connect()
    )

    # Use client...
    await client.close()
```

## API Reference

### Clasp

- `connect()` - Connect to server
- `close()` - Close connection
- `subscribe(pattern, callback)` - Subscribe to address pattern
- `on(pattern)` - Decorator for subscriptions
- `set(address, value)` - Set parameter value
- `get(address)` - Get parameter value
- `emit(address, payload)` - Emit event
- `stream(address, value)` - Send stream sample
- `bundle(messages, at)` - Send atomic bundle
- `cached(address)` - Get cached value
- `time()` - Get server time (microseconds)

### Properties

- `connected` - Check if connected
- `session_id` - Get session ID

### Callbacks

- `on_connect(callback)` - Register connect callback
- `on_disconnect(callback)` - Register disconnect callback
- `on_error(callback)` - Register error callback

## Address Patterns

CLASP supports wildcard patterns:

- `*` - Match single path segment
- `**` - Match multiple path segments

Examples:
- `/lumen/layer/*/opacity` - All layer opacities
- `/lumen/**/enabled` - All enabled properties

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

MIT

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
