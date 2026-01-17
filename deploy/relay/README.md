# CLASP Relay Server Deployment

This directory contains deployment configurations for `relay.clasp.to`, a public CLASP relay server.

## Architecture

```
Internet → TLS (443) → relay.clasp.to → clasp-router (7330)
```

The relay server runs `clasp-router` which provides:
- Full CLASP v2 protocol over WebSocket
- Binary MessagePack frames
- HELLO/WELCOME handshake
- State management with revisions
- Pattern-based subscriptions

## Local Development

### Using Docker Compose

```bash
# From repository root
docker compose -f deploy/relay/docker-compose.yml up --build

# Test connection
wscat -c ws://localhost:7330 -s clasp.v2
```

### Using Cargo

```bash
# Build and run directly
cargo run -p clasp-router-server -- --listen 0.0.0.0:7330 --name "Local Relay"

# With verbose logging
cargo run -p clasp-router-server -- --verbose
```

## Production Deployment

### DigitalOcean App Platform

1. Fork/clone the repository to your GitHub account
2. Install the DigitalOcean CLI: `brew install doctl`
3. Authenticate: `doctl auth init`
4. Deploy:

```bash
doctl apps create --spec deploy/relay/digitalocean/app.yaml
```

5. Set up DNS: Point `relay.clasp.to` to your app's URL

### Manual Docker Deployment

```bash
# Build image
docker build -t clasp-relay -f deploy/relay/Dockerfile .

# Run with TLS termination (use a reverse proxy like nginx/caddy)
docker run -d \
  --name clasp-relay \
  -p 7330:7330 \
  --restart unless-stopped \
  clasp-relay

# Or use with Caddy for automatic TLS
# In Caddyfile:
# relay.clasp.to {
#     reverse_proxy localhost:7330
# }
```

## Configuration

### Command Line Options

```
clasp-router [OPTIONS]

Options:
  -l, --listen <LISTEN>  Listen address [default: 0.0.0.0:7330]
  -n, --name <NAME>      Server name [default: CLASP Router]
  -a, --announce         Enable mDNS discovery announcement
  -c, --config <CONFIG>  Config file path
  -v, --verbose          Enable verbose logging
  -h, --help             Print help
  -V, --version          Print version
```

### Environment Variables

- `RUST_LOG` - Logging level (error, warn, info, debug, trace)

## Testing the Relay

### With wscat

```bash
# Install wscat
npm install -g wscat

# Connect with CLASP subprotocol
wscat -c wss://relay.clasp.to -s clasp.v2
```

### With the Playground

1. Open https://clasp.to/playground
2. In the connection panel, enter `wss://relay.clasp.to`
3. Click Connect
4. Try the Chat or Sensors tabs

### With JavaScript

```javascript
import { ClaspBuilder } from '@clasp-to/core';

const client = await new ClaspBuilder('wss://relay.clasp.to')
  .name('Test Client')
  .connect();

// Set a value
client.set('/test/hello', 'world');

// Subscribe to changes
client.on('/test/**', (value, address) => {
  console.log(`${address} = ${value}`);
});
```

## Monitoring

### Health Check

The server accepts any WebSocket connection as a health indicator. Use:

```bash
curl -I ws://localhost:7330
```

### Logs

```bash
# Docker Compose
docker compose -f deploy/relay/docker-compose.yml logs -f

# DigitalOcean
doctl apps logs <app-id> --follow
```

## Cost Estimate

| Provider | Tier | Monthly Cost |
|----------|------|--------------|
| DigitalOcean App Platform | basic-xxs | $5 |
| DigitalOcean Droplet | Basic $6 | $6 |
| AWS Lightsail | $5 plan | $5 |
| Fly.io | Free tier | $0 (with limits) |

## Security Notes

- The public relay does NOT enforce authentication
- Anyone can connect and send/receive messages
- Do not send sensitive data through the public relay
- For production use, deploy your own relay with JWT authentication
