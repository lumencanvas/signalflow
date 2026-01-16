# clasp-router

Message router and server for CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **Message Routing** - Route messages between connected clients
- **Pattern Matching** - Wildcard subscriptions with `*` and `**`
- **State Management** - Maintain parameter state with history
- **Session Management** - Track client connections and subscriptions

## Usage

```rust
use clasp_router::Router;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Router::new("CLASP Router");

    let addr: SocketAddr = "0.0.0.0:7330".parse()?;
    router.serve(addr).await?;

    Ok(())
}
```

## Architecture

```
┌─────────┐     ┌─────────┐     ┌─────────┐
│ Client1 │     │ Client2 │     │ Client3 │
└────┬────┘     └────┬────┘     └────┬────┘
     │               │               │
     └───────────────┼───────────────┘
                     │
              ┌──────▼──────┐
              │   Router    │
              │  (clasp-    │
              │   router)   │
              └─────────────┘
```

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
