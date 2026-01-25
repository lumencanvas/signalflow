# Manual Connection

Connect to a CLASP router using a known address when auto-discovery isn't available.

## Overview

Manual connection is useful when:
- Discovery is disabled or blocked
- Connecting to remote routers over the internet
- Connecting to routers in different network segments
- Deterministic connection is required

## Basic Connection

### JavaScript

```javascript
const { Clasp } = require('@clasp-to/core');

// WebSocket connection
const client = await Clasp.connect('ws://192.168.1.100:7330');

// With TLS
const client = await Clasp.connect('wss://192.168.1.100:7330');

// With options
const client = await Clasp.builder('ws://192.168.1.100:7330')
  .withName('my-client')
  .withTimeout(10000)
  .connect();
```

### Python

```python
from clasp import Clasp

# Basic connection
client = await Clasp.connect('ws://192.168.1.100:7330')

# With TLS
client = await Clasp.connect('wss://192.168.1.100:7330')

# With options
client = await Clasp.connect(
    'ws://192.168.1.100:7330',
    name='my-client',
    timeout=10.0
)
```

### Rust

```rust
use clasp_client::{Clasp, ClaspBuilder};

// Basic connection
let client = Clasp::connect_to("ws://192.168.1.100:7330").await?;

// With TLS (automatic with wss://)
let client = Clasp::connect_to("wss://192.168.1.100:7330").await?;

// With builder for more options
let client = ClaspBuilder::new("ws://192.168.1.100:7330")
    .name("my-client")
    .connect()
    .await?;
```

### CLI

```bash
# Connect with CLI client
clasp connect ws://192.168.1.100:7330

# With TLS
clasp connect wss://router.example.com:7330
```

## Connection URL Format

```
scheme://host:port[/path]

ws://192.168.1.100:7330      # Local WebSocket
wss://router.example.com:7330 # Secure WebSocket
```

### Supported Schemes

| Scheme | Transport | Security |
|--------|-----------|----------|
| `ws://` | WebSocket | None |
| `wss://` | WebSocket | TLS |
| `quic://` | QUIC | TLS (built-in) |

## Configuration File

Store connection settings:

```yaml
# ~/.clasp/config.yaml
default_router: ws://192.168.1.100:7330

routers:
  studio:
    url: ws://192.168.1.100:7330
    name: "Studio Workstation"
  live:
    url: wss://venue.example.com:7330
    token: "eyJhbGci..."
  home:
    url: ws://10.0.0.50:7330
```

Use named routers:

```javascript
// Connect using config name
const client = await Clasp.connect('studio');
```

```bash
# CLI with named router
clasp connect studio
```

## Environment Variables

```bash
# Set default router
export CLASP_ROUTER_URL=ws://192.168.1.100:7330

# In your code
const client = await Clasp.connect();  # Uses CLASP_ROUTER_URL
```

## Connection Options

### Timeout

```javascript
const client = await Clasp.builder('ws://192.168.1.100:7330')
  .withTimeout(5000)  // 5 second timeout
  .connect();
```

### Auto-Reconnect

```javascript
const client = await Clasp.builder('ws://192.168.1.100:7330')
  .withAutoReconnect(true)
  .withReconnectInterval(1000)  // Retry every 1 second
  .withMaxReconnectAttempts(10)
  .connect();
```

### Authentication

```javascript
const client = await Clasp.builder('ws://192.168.1.100:7330')
  .withToken('eyJhbGci...')
  .connect();
```

## Multiple Routers

Connect to multiple routers simultaneously:

```javascript
const studio = await Clasp.connect('ws://192.168.1.100:7330');
const venue = await Clasp.connect('ws://192.168.1.101:7330');

// Forward messages between routers
studio.on('/control/**', async (value, address) => {
  await venue.set(address, value);
});
```

## Connection Events

```javascript
const client = await Clasp.builder('ws://192.168.1.100:7330')
  .connect();

client.on('connected', () => {
  console.log('Connected to router');
});

client.on('disconnected', (reason) => {
  console.log('Disconnected:', reason);
});

client.on('reconnecting', (attempt) => {
  console.log('Reconnecting, attempt', attempt);
});

client.on('error', (error) => {
  console.error('Connection error:', error);
});
```

## Testing Connection

Verify connection before use:

```javascript
const client = await Clasp.builder('ws://192.168.1.100:7330')
  .connect();

// Check connection status
if (client.isConnected()) {
  console.log('Ready');
}

// Ping router
const latency = await client.ping();
console.log(`Latency: ${latency}ms`);
```

## Troubleshooting

### Connection Refused

1. Verify router is running: `clasp server status`
2. Check IP address and port are correct
3. Ensure firewall allows connections on router port

### Connection Timeout

1. Check network connectivity: `ping 192.168.1.100`
2. Verify router address is reachable
3. Increase timeout value
4. Check for network latency issues

### TLS Errors

1. Verify server certificate is valid
2. For self-signed certs, configure trust:
   ```javascript
   const client = await Clasp.builder('wss://192.168.1.100:7330')
     .withTlsConfig({ rejectUnauthorized: false })
     .connect();
   ```
3. Check certificate hostname matches connection URL

### Authentication Failed

1. Verify token is valid and not expired
2. Check token has required permissions
3. Ensure token is properly formatted

## Next Steps

- [mDNS Discovery](mdns-discovery.md)
- [Enable TLS](../security/enable-tls.md)
- [Capability Tokens](../security/capability-tokens.md)
