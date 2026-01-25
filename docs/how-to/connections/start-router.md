# Start a Router

Start a CLASP router to enable communication between clients.

## Using CLI

```bash
clasp server --port 7330
```

Output:
```
CLASP router started on ws://0.0.0.0:7330
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--port` | 7330 | WebSocket port |
| `--bind` | 0.0.0.0 | Bind address |
| `--quic-port` | - | QUIC port (requires --cert and --key) |
| `--mqtt-port` | - | MQTT server port |
| `--mqtt-namespace` | /mqtt | MQTT topic namespace prefix |
| `--osc-port` | - | OSC server port |
| `--osc-namespace` | /osc | OSC address namespace prefix |

```bash
# Custom port
clasp server --port 8080

# Bind to localhost only
clasp server --bind 127.0.0.1 --port 7330

# Enable QUIC (requires TLS)
clasp server --port 7330 --quic-port 7331 --cert cert.pem --key key.pem
```

### Multi-Protocol Server

Accept clients via multiple protocols simultaneously:

```bash
# WebSocket + MQTT
clasp server --port 7330 --mqtt-port 1883

# WebSocket + OSC
clasp server --port 7330 --osc-port 8000

# All protocols
clasp server --port 7330 --mqtt-port 1883 --osc-port 8000 --quic-port 7331 --cert cert.pem --key key.pem
```

When multiple protocols are enabled, they all share the same router state. An MQTT client publishing to `sensors/temp` can be received by a WebSocket client subscribed to `/mqtt/sensors/**`.

## Using Desktop App

The desktop app includes an embedded router:

1. Launch CLASP Bridge
2. Router starts automatically on `localhost:7330`

No configuration needed.

## Using Docker

```bash
docker run -p 7330:7330 lumencanvas/clasp-router
```

With Docker Compose:

```yaml
services:
  clasp:
    image: lumencanvas/clasp-router
    ports:
      - "7330:7330"
```

## Embedded in Your App

### Rust

```rust
use clasp_router::{Router, RouterConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = RouterConfig::default();
    let router = Router::new(config);

    // Direct state access
    router.state().set_value("/app/status", "ready", "server");

    // Accept connections
    router.serve_websocket("0.0.0.0:7330").await
}
```

### Multi-Protocol (Rust)

```rust
use clasp_router::{Router, RouterConfig, MultiProtocolConfig, MqttServerConfig, OscServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Router::new(RouterConfig::default());

    let config = MultiProtocolConfig {
        websocket_addr: Some("0.0.0.0:7330".into()),
        mqtt: Some(MqttServerConfig {
            bind_addr: "0.0.0.0:1883".into(),
            namespace: "/mqtt".into(),
            ..Default::default()
        }),
        osc: Some(OscServerConfig {
            bind_addr: "0.0.0.0:8000".into(),
            namespace: "/osc".into(),
            ..Default::default()
        }),
        ..Default::default()
    };

    // All protocols share the same router state
    router.serve_all(config).await
}
```

### Node.js

```javascript
import { ClaspRouter } from '@clasp-to/core';

const router = new ClaspRouter({ port: 7330 });
await router.start();

// Direct access
router.set('/app/status', 'ready');
```

## Verify Router is Running

```bash
# Check port is listening
lsof -i :7330

# Test with a client
node -e "
  const ws = new (require('ws'))('ws://localhost:7330');
  ws.on('open', () => { console.log('Connected'); ws.close(); });
"
```

## Configuration

### Environment Variables

```bash
RUST_LOG=info clasp server
```

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level (error, warn, info, debug, trace) |
| `CLASP_PORT` | Default port |
| `CLASP_BIND` | Default bind address |

### Config File (Advanced)

Create `clasp.toml`:

```toml
[router]
port = 7330
bind = "0.0.0.0"

[router.quic]
enabled = true
port = 7331

[router.tls]
cert = "/path/to/cert.pem"
key = "/path/to/key.pem"
```

Run with:
```bash
clasp server --config clasp.toml
```

## Troubleshooting

### "Address already in use"

Another process is using the port:

```bash
# Find what's using it
lsof -i :7330

# Use different port
clasp server --port 7331
```

### "Permission denied" (port < 1024)

Use a higher port or run with elevated privileges:

```bash
# Use high port (recommended)
clasp server --port 7330

# Or on Linux with capabilities
sudo setcap 'cap_net_bind_service=+ep' $(which clasp)
```

## Next Steps

- [Connect a Client](connect-client.md)
- [Add Protocol Bridges](add-osc.md)
- [Router Configuration Reference](../../reference/configuration/router-config.md)
