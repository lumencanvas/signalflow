# clasp-client

Async client library for CLASP (Creative Low-Latency Application Streaming Protocol).

## Usage

```rust
use clasp_client::{Clasp, ClaspBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect using builder
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("My App")
        .connect()
        .await?;

    // Set a parameter
    client.set("/lights/front/brightness", 0.75.into()).await?;

    // Get a parameter
    let value = client.get("/lights/front/brightness").await?;
    println!("Brightness: {:?}", value);

    // Subscribe to changes
    let _unsub = client.subscribe("/lights/*", |value, addr| {
        println!("{} = {:?}", addr, value);
    }).await?;

    // Close connection
    client.close().await?;
    Ok(())
}
```

## Features

- Async/await API with Tokio
- WebSocket transport with automatic reconnection
- Time synchronization with server
- Pattern-based subscriptions with wildcards

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
