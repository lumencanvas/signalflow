# mDNS Discovery

Automatically discover CLASP routers on your local network using mDNS (Bonjour/Avahi).

## Overview

mDNS allows clients to find routers without knowing their IP addresses. Routers advertise themselves as `_clasp._tcp.local`.

## Enable mDNS on Router

### CLI

```bash
clasp server --port 7330 --mdns
```

### Configuration File

```yaml
# clasp.yaml
server:
  port: 7330
  discovery:
    mdns:
      enabled: true
      name: "Studio Router"  # Optional friendly name
```

### Programmatic (Rust)

```rust
use clasp_router::{Router, RouterConfig};

let config = RouterConfig {
    name: "Studio Router".into(),
    ..Default::default()
};

let router = Router::new(config);
router.serve_websocket("0.0.0.0:7330").await?;
```

## Discover Routers

### CLI

```bash
# List available routers
clasp discover

# Output:
# NAME             ADDRESS              PORT
# Studio Router    192.168.1.100        7330
# Living Room      192.168.1.101        7330
```

### JavaScript

```javascript
const { Clasp, Discovery } = require('@clasp-to/core');

// Discover routers
const routers = await Discovery.findRouters();
console.log('Found routers:', routers);
// [{ name: 'Studio Router', host: '192.168.1.100', port: 7330 }]

// Connect to first found router
const client = await Clasp.discover();

// Or with options
const client = await Clasp.discover({
  timeout: 5000,           // Search timeout in ms
  preferName: 'Studio'     // Prefer router with matching name
});
```

### Python

```python
from clasp import Clasp, discover_routers

# Find all routers
routers = discover_routers(timeout=5.0)
for router in routers:
    print(f"{router.name}: {router.host}:{router.port}")

# Connect to first found
client = await Clasp.discover()

# Connect by name
client = await Clasp.discover(name="Studio Router")
```

### Rust

```rust
use clasp_discovery::MdnsDiscovery;

let discovery = MdnsDiscovery::new();
let routers = discovery.find_routers(Duration::from_secs(5)).await?;

for router in routers {
    println!("{}: {}:{}", router.name, router.host, router.port);
}
```

## Service Record Details

CLASP routers advertise with:

- **Service type**: `_clasp._tcp.local`
- **Port**: Router WebSocket port
- **TXT records**:
  - `version`: CLASP protocol version
  - `name`: Friendly router name
  - `secure`: "true" if TLS enabled

## Filtering Results

```javascript
// Find only secure routers
const secureRouters = await Discovery.findRouters({
  filter: (router) => router.secure === true
});

// Find router by name pattern
const router = await Discovery.findRouter({
  filter: (r) => r.name.includes('Studio')
});
```

## Continuous Discovery

Monitor for routers appearing/disappearing:

```javascript
const discovery = new Discovery();

discovery.on('found', (router) => {
  console.log('Router appeared:', router.name);
});

discovery.on('lost', (router) => {
  console.log('Router disappeared:', router.name);
});

discovery.start();

// Later
discovery.stop();
```

## Platform Requirements

### macOS

mDNS works out of the box (Bonjour built-in).

### Linux

Install Avahi:

```bash
sudo apt install avahi-daemon avahi-utils
sudo systemctl enable avahi-daemon
sudo systemctl start avahi-daemon
```

### Windows

Install Bonjour:
- Included with iTunes
- Or download Bonjour SDK from Apple

## Troubleshooting

### Router Not Found

1. Verify router is running with `--mdns` flag
2. Check client and router are on same subnet
3. Verify mDNS service is running (Bonjour/Avahi)
4. Check firewall allows UDP port 5353

### Multiple Routers with Same Name

Each router should have a unique name. Configure with:

```bash
clasp server --mdns --mdns-name "Unique Name"
```

### Discovery Slow

1. mDNS needs time to propagate (~2-5 seconds)
2. Increase discovery timeout
3. Check for network congestion

## Next Steps

- [UDP Broadcast Discovery](udp-broadcast.md)
- [Manual Connection](manual-connection.md)
- [Start Router](../connections/start-router.md)
