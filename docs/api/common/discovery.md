## Discovery

CLASP favors **zero‑config discovery** when possible, with explicit configuration as a reliable fallback.

### LAN Discovery

Typical discovery mechanisms include:

- **mDNS/Bonjour**: `_clasp._tcp.local` service for routers on the local network.
- **UDP broadcast**: A simple discovery frame on a well‑known port (e.g. `7331`).

Routers and desktop tools can advertise themselves over mDNS and/or UDP; embedded and browser clients then:

1. Discover available routers.
2. Present choices to the user or auto‑select based on policy.

### WAN Discovery (Rendezvous)

For devices across the internet, CLASP provides a **rendezvous server** for WAN discovery:

- **REST API**: Register, discover, refresh, and unregister devices via HTTP.
- **Automatic keepalive**: Clients automatically refresh their registration before TTL expires.
- **Tag filtering**: Discover devices by tag (e.g., "studio", "live", "dev").

#### Rendezvous Configuration

```rust
use clasp_discovery::{Discovery, DiscoveryConfig, DeviceRegistration};
use std::time::Duration;

let config = DiscoveryConfig {
    mdns: true,
    broadcast: true,
    rendezvous_url: Some("https://rendezvous.example.com".into()),
    rendezvous_refresh_interval: Duration::from_secs(120), // Refresh every 2 minutes
    rendezvous_tag: Some("studio".into()), // Filter by tag
    ..Default::default()
};

let mut discovery = Discovery::with_config(config);

// Register this device with the rendezvous server
discovery.register_with_rendezvous(DeviceRegistration {
    name: "My Device".into(),
    endpoints: [("ws".into(), "wss://my-device.local:7330".into())].into(),
    tags: vec!["studio".into()],
    ..Default::default()
});
```

#### Cascade Discovery

Use `discover_all()` to try all discovery methods in sequence:

```rust
// Tries: mDNS → broadcast → rendezvous
let devices = discovery.discover_all().await?;
```

Or use `discover_wan()` for rendezvous-only discovery:

```rust
let wan_devices = discovery.discover_wan().await?;
```

### Browser Considerations

Browsers cannot do raw mDNS or arbitrary UDP:

- Browser clients usually connect to a known WebSocket endpoint (`wss://host:7330/clasp`).
- A separate discovery UI or rendezvous service can provide that endpoint.
- The rendezvous REST API is browser-accessible via fetch/XHR.

### Manual Configuration

For constrained or locked‑down environments (corporate networks, WAN scenarios), discovery may be disabled entirely. In those cases:

- Users configure router addresses explicitly.
- P2P setups typically use a **rendezvous/router** for signaling but not for data.

Language‑specific docs show how to opt into discovery helpers where they exist, and how to fall back to manual configuration when they do not.

