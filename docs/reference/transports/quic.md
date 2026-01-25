# QUIC Transport

High-performance transport for CLASP.

## Overview

QUIC (Quick UDP Internet Connections) provides lower latency than WebSocket with built-in encryption and multiplexing.

## Connection URLs

```
quic://host:port   # QUIC (TLS required)
```

## Features

| Feature | Support |
|---------|---------|
| Bidirectional | Yes |
| Reliable delivery | Yes |
| Ordered delivery | Per-stream |
| Browser support | HTTP/3 only |
| TLS encryption | Required |
| Proxy compatible | Limited |
| Connection overhead | Low |
| Latency | Very Low |

## Advantages

- **0-RTT connection**: Resumed connections skip handshake
- **Head-of-line blocking avoidance**: Independent streams
- **Connection migration**: Survives IP changes
- **Built-in encryption**: TLS 1.3 mandatory
- **Multiplexing**: Multiple streams per connection

## Server Configuration

```yaml
server:
  port: 7330
  quic:
    enabled: true
    cert: /path/to/cert.pem
    key: /path/to/key.pem
    max_streams: 100
```

### CLI

```bash
clasp server --port 7330 \
  --quic \
  --tls-cert cert.pem \
  --tls-key key.pem
```

## Client Usage

### Rust

```rust
use clasp_client::Clasp;

let client = Clasp::connect_to("quic://localhost:7330").await?;
```

### JavaScript (Node.js)

```javascript
// Requires QUIC-capable runtime
const client = await Clasp.connect('quic://localhost:7330', {
  tls: {
    ca: fs.readFileSync('ca.pem')
  }
});
```

## Streams

QUIC multiplexes multiple streams over one connection:

```rust
// Open additional streams for different purposes
let control_stream = client.open_stream().await?;
let data_stream = client.open_stream().await?;

// Streams are independent - blocking one doesn't affect others
control_stream.send(control_msg).await?;
data_stream.send(data_msg).await?;
```

## 0-RTT

Resume connections without handshake:

```rust
use clasp_client::{Clasp, ClaspBuilder};

// First connection stores session ticket
let client = Clasp::connect_to("quic://localhost:7330").await?;
// Note: Session ticket API is transport-specific
client.close().await;

// Resumed connection
let client = ClaspBuilder::new("quic://localhost:7330")
    .name("my-client")
    .connect()
    .await?;
```

## Connection Migration

QUIC handles network changes gracefully:

```rust
// Connection survives WiFi → cellular
// No reconnection needed for IP changes
```

## Performance

### Typical Metrics

- Connection time: ~10-30ms (with 0-RTT: <5ms)
- Message latency: ~0.5-3ms
- Throughput: 100,000+ msg/sec
- Streams: 100+ per connection

### Configuration

```yaml
server:
  quic:
    max_streams: 100
    max_idle_timeout: 30s
    initial_window: 65536
    max_window: 16777216
```

## TLS Configuration

QUIC requires TLS 1.3:

```yaml
server:
  quic:
    cert: /path/to/cert.pem
    key: /path/to/key.pem
    alpn: ["clasp"]  # Application protocol
```

### Self-Signed Certificates

For development:

```bash
# Generate self-signed cert
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes \
  -subj "/CN=localhost"
```

## Comparison with WebSocket

| Aspect | WebSocket | QUIC |
|--------|-----------|------|
| Protocol | TCP | UDP |
| Encryption | Optional | Required |
| Connection setup | 2-3 RTT | 1 RTT (0 with resumption) |
| Head-of-line blocking | Yes | No |
| Browser support | Full | HTTP/3 only |
| Firewall traversal | Easy | May be blocked |

## When to Use QUIC

**Use QUIC when:**
- Low latency is critical
- Connection resumption is common
- Multiple independent data streams needed
- Network conditions change (mobile)

**Use WebSocket when:**
- Browser compatibility required
- Proxy/firewall traversal needed
- Simpler deployment desired

## Troubleshooting

### Connection Fails

1. Verify TLS certificates are valid
2. Check UDP port is open (firewalls may block UDP)
3. Ensure server has QUIC enabled

### High Latency

1. Check network MTU (QUIC is UDP-based)
2. Verify no packet loss
3. Review congestion control settings

## See Also

- [WebSocket Transport](websocket.md)
- [Enable TLS](../../how-to/security/enable-tls.md)
- [Performance Tuning](../../how-to/advanced/performance-tuning.md)
