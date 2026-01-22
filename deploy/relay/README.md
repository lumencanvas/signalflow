# CLASP Relay Server Deployment

This directory contains a **standalone** CLASP relay server that uses published crates from crates.io.

## Quick Start

### Option 1: Docker (Recommended)

```bash
# Build
cd deploy/relay
docker build -t clasp-relay .

# Run
docker run -p 7330:7330 clasp-relay

# Test
wscat -c ws://localhost:7330 -s clasp
```

### Option 2: Cargo

```bash
cd deploy/relay
cargo run --release
```

### Option 3: DigitalOcean App Platform

```bash
# Install doctl
brew install doctl

# Authenticate
doctl auth init

# Deploy
doctl apps create --spec deploy/relay/digitalocean/app.yaml
```

## Architecture

```
Internet → TLS (443) → DigitalOcean → clasp-relay (7330)
```

The relay runs a CLASP router that provides:
- CLASP v3 binary protocol over WebSocket
- State management with revisions
- Pattern-based subscriptions (`*`, `**`)
- No authentication (public relay)

## Development vs Production

| | Production | Development |
|---|---|---|
| **Dockerfile** | `Dockerfile` | `Dockerfile.dev` |
| **Crates** | crates.io | Local workspace |
| **Build from** | `deploy/relay/` | Repository root |

### Development Build (using monorepo)

```bash
# From repository root
docker build -f deploy/relay/Dockerfile.dev -t clasp-relay-dev .
```

## Configuration

### CLI Options

```
clasp-relay [OPTIONS]

Options:
  -p, --port <PORT>    Listen port [default: 7330]
      --host <HOST>    Listen host [default: 0.0.0.0]
  -n, --name <NAME>    Server name [default: CLASP Relay]
  -v, --verbose        Enable verbose logging
  -h, --help           Print help
  -V, --version        Print version
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level: error, warn, info, debug, trace |

## Connecting

### JavaScript

```javascript
import { ClaspBuilder } from '@clasp-to/core';

const client = await new ClaspBuilder('wss://relay.clasp.to')
  .name('My App')
  .connect();

client.set('/hello', 'world');
client.on('/hello', (value) => console.log(value));
```

### Python

```python
from clasp import Clasp

client = Clasp('wss://relay.clasp.to')
client.connect()

client.set('/hello', 'world')
client.on('/hello', print)
```

### Rust

```rust
use clasp_client::Clasp;

let client = Clasp::connect("wss://relay.clasp.to").await?;
client.set("/hello", "world").await?;
client.subscribe("/hello", |value, _| println!("{:?}", value)).await?;
```

### Embedded (ESP32)

```rust
use clasp_embedded::{Client, Value};

let mut client = Client::new();

// Prepare frame
let frame = client.prepare_set("/sensor/temp", Value::Float(25.5));

// Send via your transport (WebSocket, HTTP, etc.)
websocket.send(frame);
```

## Cost Estimate

| Provider | Tier | Monthly |
|----------|------|---------|
| DigitalOcean App Platform | basic-xxs | $5 |
| DigitalOcean Droplet | $6/mo | $6 |
| AWS Lightsail | $5 plan | $5 |
| Fly.io | Free tier | $0 |

## Security Notes

⚠️ The public relay does NOT enforce authentication:
- Anyone can connect and send/receive messages
- Do not send sensitive data through public relay
- For production, deploy your own relay with authentication

## Monitoring

### Health Check

The server responds to any WebSocket connection attempt as healthy.

### Logs

```bash
# Docker
docker logs clasp-relay -f

# DigitalOcean
doctl apps logs <app-id> --follow
```

## Troubleshooting

### "Connection refused"

1. Check the relay is running: `docker ps`
2. Check the port is exposed: `docker port clasp-relay`
3. Check firewall rules

### "Upgrade failed"

WebSocket requires HTTP Upgrade header. Ensure your client uses `ws://` or `wss://`.

### Build fails on DigitalOcean

1. Check `source_dir` in app.yaml points to `deploy/relay`
2. Ensure Cargo.toml exists in deploy/relay/
3. Check build logs: `doctl apps logs <app-id> --type build`
