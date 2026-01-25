# clasp-discovery (Rust)

Discovery mechanisms for finding CLASP routers.

## Overview

`clasp-discovery` provides automatic router discovery using mDNS and UDP broadcast.

```toml
[dependencies]
clasp-discovery = "3.1"
```

## Features

```toml
# All discovery methods
clasp-discovery = { version = "3.1", features = ["full"] }

# Individual methods
clasp-discovery = { version = "3.1", features = ["mdns", "udp"] }
```

## mDNS Discovery

### Find Routers

```rust
use clasp_discovery::mdns::MdnsDiscovery;
use std::time::Duration;

let discovery = MdnsDiscovery::new();

// Find all routers (blocking for timeout)
let routers = discovery.find_routers(Duration::from_secs(5)).await?;

for router in routers {
    println!("Found: {} at {}:{}", router.name, router.host, router.port);
}
```

### Find Single Router

```rust
// Find first available router
let router = discovery.find_router(Duration::from_secs(5)).await?;
println!("Connecting to {}:{}", router.host, router.port);
```

### Continuous Discovery

```rust
use clasp_discovery::mdns::{MdnsDiscovery, DiscoveryEvent};

let discovery = MdnsDiscovery::new();
let mut events = discovery.subscribe();

tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            DiscoveryEvent::Found(router) => {
                println!("Router appeared: {}", router.name);
            }
            DiscoveryEvent::Lost(router) => {
                println!("Router disappeared: {}", router.name);
            }
        }
    }
});

discovery.start().await?;
```

### Advertise Router

```rust
use clasp_discovery::mdns::MdnsAdvertiser;

let advertiser = MdnsAdvertiser::new(MdnsConfig {
    name: "My Router".into(),
    port: 7330,
    secure: false,
    metadata: HashMap::new(),
});

advertiser.start().await?;

// Stop advertising
advertiser.stop().await?;
```

## UDP Broadcast Discovery

### Find Routers

```rust
use clasp_discovery::udp::UdpDiscovery;

let discovery = UdpDiscovery::new(UdpConfig {
    port: 7331,
    broadcast_addr: "255.255.255.255".parse()?,
});

let routers = discovery.find_routers(Duration::from_secs(3)).await?;
```

### Specific Subnet

```rust
let discovery = UdpDiscovery::new(UdpConfig {
    port: 7331,
    broadcast_addr: "192.168.1.255".parse()?,  // Class C subnet
});
```

### Respond to Discovery

```rust
use clasp_discovery::udp::UdpResponder;

let responder = UdpResponder::new(ResponderConfig {
    port: 7331,
    router_port: 7330,
    name: "My Router".into(),
});

responder.start().await?;
```

## Router Info

```rust
use clasp_discovery::RouterInfo;

pub struct RouterInfo {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub secure: bool,
    pub version: String,
    pub metadata: HashMap<String, String>,
}

impl RouterInfo {
    /// Get WebSocket URL
    pub fn ws_url(&self) -> String {
        let scheme = if self.secure { "wss" } else { "ws" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }
}
```

## Combined Discovery

Use multiple methods together:

```rust
use clasp_discovery::{Discovery, DiscoveryConfig};

let discovery = Discovery::new(DiscoveryConfig {
    mdns_enabled: true,
    udp_enabled: true,
    udp_port: 7331,
    timeout: Duration::from_secs(5),
});

// Tries mDNS first, falls back to UDP
let routers = discovery.find_routers().await?;
```

## Filtering

```rust
// Find by name
let router = discovery.find_router_by_name("Studio Router").await?;

// Find secure routers only
let routers = discovery.find_routers_filtered(|r| r.secure).await?;

// Find by metadata
let routers = discovery.find_routers_filtered(|r| {
    r.metadata.get("environment") == Some(&"production".into())
}).await?;
```

## Error Handling

```rust
use clasp_discovery::Error;

match discovery.find_router(timeout).await {
    Ok(router) => println!("Found: {}", router.name),
    Err(Error::NoRoutersFound) => println!("No routers found"),
    Err(Error::Timeout) => println!("Discovery timed out"),
    Err(Error::NetworkError(e)) => println!("Network error: {}", e),
    Err(e) => println!("Error: {:?}", e),
}
```

## Configuration

### mDNS Config

```rust
pub struct MdnsConfig {
    /// Service name to advertise/discover
    pub service_type: String,  // Default: "_clasp._tcp.local"

    /// Router name
    pub name: String,

    /// Port to advertise
    pub port: u16,

    /// Whether TLS is enabled
    pub secure: bool,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}
```

### UDP Config

```rust
pub struct UdpConfig {
    /// Discovery port
    pub port: u16,  // Default: 7331

    /// Broadcast address
    pub broadcast_addr: SocketAddr,

    /// Response timeout
    pub timeout: Duration,
}
```

## Platform Notes

### macOS

mDNS works out of the box (Bonjour).

### Linux

Requires Avahi:

```bash
sudo apt install avahi-daemon libavahi-client-dev
```

### Windows

Requires Bonjour SDK or relies on UDP fallback.

## See Also

- [mDNS Discovery](../../../how-to/discovery/mdns-discovery.md)
- [UDP Broadcast](../../../how-to/discovery/udp-broadcast.md)
- [clasp-client](clasp-client.md) - Client library
