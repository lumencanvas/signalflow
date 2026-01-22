# clasp-router

Message router and server for CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **Message Routing** - Route messages between connected clients
- **Pattern Matching** - Wildcard subscriptions with `*` and `**`
- **State Management** - Parameter state with revision tracking
- **Session Management** - Track client connections and subscriptions
- **Multiple Transports** - WebSocket, QUIC, UDP

## Usage

```rust
use clasp_router::{Router, RouterConfig};
use clasp_core::SecurityMode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Router::new(RouterConfig {
        name: "My Router".into(),
        max_sessions: 100,
        session_timeout: 60,
        features: vec!["param".into(), "event".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 100,
    });

    // Serve on WebSocket
    router.serve_websocket("0.0.0.0:7330").await?;
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
              │  - State    │
              │  - Fanout   │
              │  - Sessions │
              └─────────────┘
```

## Performance

| Metric | Value |
|--------|-------|
| E2E throughput | 173k msg/s |
| Fanout 100 subs | 175k deliveries/s |
| Events (no state) | 259k msg/s |
| Late-joiner replay | Yes (chunked snapshots) |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio)
