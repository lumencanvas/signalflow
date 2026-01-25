# UDP Broadcast Discovery

Discover CLASP routers using UDP broadcast when mDNS is unavailable.

## Overview

UDP broadcast provides a lightweight alternative to mDNS for router discovery. It works by sending broadcast packets that routers respond to.

## Enable UDP Discovery on Router

### CLI

```bash
clasp server --port 7330 --discovery-port 7331
```

### Configuration

```yaml
# clasp.yaml
server:
  port: 7330
  discovery:
    udp:
      enabled: true
      port: 7331
```

### Rust

```rust
use clasp_router::{Router, RouterConfig};

let config = RouterConfig::default();
let router = Router::new(config);

// Start router with WebSocket
router.serve_websocket("0.0.0.0:7330").await?;
```

## Discover Routers

### CLI

```bash
# Broadcast discovery
clasp discover --udp
```

### JavaScript

```javascript
const { Discovery } = require('@clasp-to/core');

// UDP discovery
const routers = await Discovery.findRouters({
  method: 'udp',
  port: 7331,
  timeout: 3000
});

// Broadcast to specific subnet
const routers = await Discovery.findRouters({
  method: 'udp',
  broadcast: '192.168.1.255',
  port: 7331
});
```

### Python

```python
from clasp import discover_routers

routers = discover_routers(method='udp', port=7331, timeout=3.0)
```

## Protocol Details

Discovery uses a simple request/response protocol:

### Discovery Request

Client sends UDP broadcast:

```
CLASP-DISCOVER\n
```

### Discovery Response

Router responds with:

```json
{
  "name": "Studio Router",
  "host": "192.168.1.100",
  "port": 7330,
  "version": "1.0",
  "secure": false
}
```

## Network Configuration

### Broadcast Address

By default, discovery broadcasts to `255.255.255.255`. For specific subnets:

```javascript
const routers = await Discovery.findRouters({
  method: 'udp',
  broadcast: '10.0.0.255'  // Class A subnet
});
```

### Port Selection

Choose a discovery port that doesn't conflict with other services:

- Default: 7331
- Range: 1024-65535 (non-privileged)

## Firewall Configuration

Allow UDP broadcast:

### Linux (iptables)

```bash
sudo iptables -A INPUT -p udp --dport 7331 -j ACCEPT
sudo iptables -A INPUT -p udp --sport 7331 -j ACCEPT
```

### macOS

UDP broadcast typically works without configuration.

### Windows

Allow through Windows Firewall:

```powershell
netsh advfirewall firewall add rule name="CLASP Discovery" dir=in action=allow protocol=udp localport=7331
```

## Comparison with mDNS

| Feature | mDNS | UDP Broadcast |
|---------|------|---------------|
| Setup | Requires service (Avahi/Bonjour) | None |
| Reliability | High | Depends on network |
| Cross-subnet | Limited | No |
| Response time | ~2-5s | ~100ms |
| Standard | Yes (RFC 6762) | CLASP-specific |

## When to Use UDP Discovery

- mDNS is unavailable or unreliable
- Need faster discovery
- Simple networks without mDNS infrastructure
- Embedded devices without mDNS stack

## Troubleshooting

### No Routers Found

1. Verify router has UDP discovery enabled
2. Check broadcast address is correct for your subnet
3. Confirm firewall allows UDP on discovery port
4. Try increasing timeout

### Discovery Works Locally Only

1. Routers can't be discovered across subnets
2. Check if network allows broadcast traffic
3. Consider using mDNS for cross-subnet discovery

### Intermittent Results

1. Increase timeout
2. Send multiple discovery requests
3. Check for network congestion
4. Verify router is responding (check logs)

## Next Steps

- [mDNS Discovery](mdns-discovery.md)
- [Manual Connection](manual-connection.md)
- [Start Router](../connections/start-router.md)
