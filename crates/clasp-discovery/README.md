# clasp-discovery

Network discovery for CLASP (Creative Low-Latency Application Streaming Protocol) devices and servers.

## Features

- **mDNS/DNS-SD** - Zero-configuration discovery on local networks
- **UDP Broadcast** - Fallback discovery when mDNS is unavailable
- **Service Announcement** - Advertise CLASP services to the network

## Usage

```rust
use clasp_discovery::Discovery;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let discovery = Discovery::new().await?;

    // Start discovering devices
    discovery.start().await?;

    // Wait for devices
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Get discovered devices
    for device in discovery.devices() {
        println!("Found: {} at {:?}", device.name, device.endpoints);
    }

    Ok(())
}
```

## Service Announcement

```rust
use clasp_discovery::Discovery;

let discovery = Discovery::new().await?;
discovery.announce("My CLASP Server", 7330).await?;
```

## mDNS Service Type

CLASP uses the service type `_clasp._tcp.local` for mDNS discovery.

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
