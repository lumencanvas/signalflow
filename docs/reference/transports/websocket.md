# WebSocket Transport

Primary transport for CLASP communication.

## Overview

WebSocket is the default and most widely supported transport for CLASP. It provides reliable, bidirectional communication over TCP with broad browser and platform support.

## Connection URLs

```
ws://host:port     # Unencrypted WebSocket
wss://host:port    # TLS-encrypted WebSocket
```

## Features

| Feature | Support |
|---------|---------|
| Bidirectional | Yes |
| Reliable delivery | Yes |
| Ordered delivery | Yes |
| Browser support | Yes |
| TLS encryption | Yes |
| Proxy compatible | Yes |
| Connection overhead | Medium |
| Latency | Low |

## Client Usage

### JavaScript

```javascript
const client = await Clasp.connect('ws://localhost:7330');
```

### Python

```python
client = await Clasp.connect('ws://localhost:7330')
```

### Rust

```rust
use clasp_client::Clasp;

let client = Clasp::connect_to("ws://localhost:7330").await?;
```

## Server Configuration

### Basic

```yaml
server:
  port: 7330
  bind: "0.0.0.0"
```

### With TLS

```yaml
server:
  port: 7330
  tls:
    enabled: true
    cert: /path/to/cert.pem
    key: /path/to/key.pem
```

## WebSocket Frame Format

CLASP messages are sent as WebSocket binary frames:

```
┌─────────────────────────────────────────┐
│ WebSocket Frame (Binary)                │
├─────────────────────────────────────────┤
│ ┌─────────────────────────────────────┐ │
│ │ CLASP Frame Header (4 bytes)        │ │
│ ├─────────────────────────────────────┤ │
│ │ CLASP Message Payload               │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## Connection Lifecycle

1. **Connect**: TCP handshake, WebSocket upgrade
2. **Hello**: Client sends HELLO, server responds
3. **Active**: Bidirectional message exchange
4. **Close**: Graceful close or timeout

## Keepalive

WebSocket ping/pong for connection health:

```yaml
server:
  websocket:
    ping_interval: 30  # seconds
    pong_timeout: 10   # seconds
```

## Connection Limits

```yaml
server:
  max_connections: 10000
  max_connections_per_ip: 100
  max_frame_size: 65536  # bytes
```

## Proxy Configuration

### nginx

```nginx
location / {
    proxy_pass http://localhost:7330;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_read_timeout 3600s;
}
```

### HAProxy

```
frontend ws
    bind *:443 ssl crt /etc/ssl/cert.pem
    default_backend clasp

backend clasp
    server router1 127.0.0.1:7330
    timeout tunnel 3600s
```

## Performance

### Typical Metrics

- Connection time: ~5-20ms (local), ~50-200ms (remote)
- Message latency: ~1-5ms (local)
- Throughput: 50,000+ msg/sec per connection
- Memory: ~10KB per connection

### Optimization

```yaml
server:
  websocket:
    compression: true  # Per-message deflate
    max_frame_size: 65536
    buffer_size: 16384
```

## Comparison with Other Transports

| Transport | Latency | Throughput | Browser | Reliability |
|-----------|---------|------------|---------|-------------|
| WebSocket | Low | High | Yes | High |
| QUIC | Very Low | Very High | No* | High |
| UDP | Lowest | Highest | No | Best-effort |
| WebRTC | Low | High | Yes | Configurable |

*QUIC browser support via HTTP/3

## See Also

- [Connect Client](../../how-to/connections/connect-client.md)
- [Enable TLS](../../how-to/security/enable-tls.md)
- [QUIC Transport](quic.md)
