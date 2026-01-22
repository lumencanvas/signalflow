# CLASP Examples

This directory contains example code demonstrating how to use CLASP in various scenarios.

## JavaScript Examples

### simple-publisher.js

Demonstrates publishing values and events to a CLASP server.

```bash
cd examples/js
npm install @clasp-to/core
node simple-publisher.js
```

Features demonstrated:
- Setting parameter values
- Emitting events
- Streaming high-rate data
- Atomic bundles
- Scheduled bundles

### simple-subscriber.js

Demonstrates subscribing to values and events from a CLASP server.

```bash
cd examples/js
npm install @clasp-to/core
node simple-subscriber.js
```

Features demonstrated:
- Subscribing to specific addresses
- Wildcard subscriptions (`*` and `**`)
- Rate-limited subscriptions
- Change threshold filtering (epsilon)
- Getting values (async)
- Checking cached values (sync)
- Unsubscribing

### embedded-server.js

Demonstrates integrating CLASP with your Node.js application.

```bash
# Start router first
cargo run -p clasp-router-server -- --listen 0.0.0.0:7330

# Then run the example
cd examples/js
npm install @clasp-to/core
node embedded-server.js
```

Features demonstrated:
- Connecting to CLASP from your app
- Publishing application state (CPU, memory, sensors)
- Subscribing to commands from other clients

## Python Examples

### embedded_server.py

Demonstrates integrating CLASP with your Python application.

```bash
# Start router first
cargo run -p clasp-router-server -- --listen 0.0.0.0:7330

# Then run the example
pip install clasp-to
python examples/python/embedded_server.py
```

Features demonstrated:
- Async connection to CLASP
- Publishing sensor data periodically
- Command handling via subscriptions

## Rust Examples

### basic-client.rs

Comprehensive Rust client example.

```bash
cargo run --example basic-client
```

### embedded-server.rs

Demonstrates embedding a CLASP server in your Rust application.

```bash
cargo run --example embedded-server
```

Features demonstrated:
- Running CLASP router alongside your business logic
- Publishing data from your application to connected clients
- Custom routing and state management

Add to your project:

```toml
[dependencies]
clasp-router = "0.1"  # For server
clasp-client = "0.1"  # For client
```

Features demonstrated (basic-client):
- Builder pattern for client creation
- Setting parameters
- Subscribing with callbacks
- Emitting events
- Streaming data
- Getting values
- Atomic and scheduled bundles

## Docker Compose

### docker-compose.yml

Complete development environment with CLASP Router and MQTT broker.

```bash
# Start basic setup
docker-compose up -d clasp-router mqtt

# Start with Redis for distributed state
docker-compose --profile distributed up -d

# Stop
docker-compose down
```

Services:
- **clasp-router**: Core CLASP message router (port 7330)
- **mqtt**: Mosquitto MQTT broker (port 1883)
- **redis**: Redis for distributed state (port 6379, optional)

## Environment Variables

All examples support the following environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `CLASP_URL` | `ws://localhost:7330` | CLASP server WebSocket URL |

## Running a CLASP Server

To run these examples, you need a CLASP server. Options:

1. **Desktop App**: Download from [releases](https://github.com/lumencanvas/clasp/releases)

2. **Docker**:
   ```bash
   docker run -p 7330:7330 lumencanvas/clasp-router
   ```

3. **From Source**:
   ```bash
   cargo run -p clasp-router-server
   ```

## More Examples

For more complex integration examples, see:
- [TouchOSC Integration](../docs/integrations/touchosc.md)
- [Resolume Integration](../docs/integrations/resolume.md)
- [QLab Integration](../docs/integrations/qlab.md)
