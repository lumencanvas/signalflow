# clasp-router

Message router and server for CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **Message Routing** - Route messages between connected clients
- **Pattern Matching** - Wildcard subscriptions with `*` and `**`
- **State Management** - Parameter state with revision tracking
- **Session Management** - Track client connections and subscriptions
- **Multiple Transports** - WebSocket, QUIC, TCP
- **Protocol Adapters** - Accept MQTT and OSC clients directly (optional features)
- **Rate Limiting** - Configurable per-client message rate limits
- **Gesture Coalescing** - Reduce bandwidth for high-frequency gesture streams

## Installation

```toml
[dependencies]
clasp-router = "3.1"

# Optional: Enable protocol adapters
clasp-router = { version = "3.1", features = ["mqtt-server", "osc-server"] }
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `websocket` | WebSocket transport (default) |
| `quic` | QUIC transport with built-in TLS |
| `tcp` | Raw TCP transport |
| `mqtt-server` | Accept MQTT clients directly |
| `osc-server` | Accept OSC clients via UDP |
| `full` | All features enabled |

## Basic Usage

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
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        max_messages_per_second: 1000,
        rate_limiting_enabled: true,
    });

    // Serve on WebSocket
    router.serve_websocket("0.0.0.0:7330").await?;
    Ok(())
}
```

## Multi-Protocol Server

Serve multiple protocols simultaneously with shared state:

```rust
use clasp_router::{Router, RouterConfig, MultiProtocolConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Router::new(RouterConfig::default());

    let config = MultiProtocolConfig {
        websocket_addr: Some("0.0.0.0:7330".into()),
        #[cfg(feature = "mqtt-server")]
        mqtt: Some(clasp_router::MqttServerConfig {
            bind_addr: "0.0.0.0:1883".into(),
            namespace: "/mqtt".into(),
            ..Default::default()
        }),
        #[cfg(feature = "osc-server")]
        osc: Some(clasp_router::OscServerConfig {
            bind_addr: "0.0.0.0:8000".into(),
            namespace: "/osc".into(),
            ..Default::default()
        }),
        ..Default::default()
    };

    // All protocols share the same router state
    router.serve_all(config).await?;
    Ok(())
}
```

## Protocol Adapters

### MQTT Server Adapter

Accept MQTT clients directly without an external broker:

```rust
use clasp_router::MqttServerConfig;

let mqtt_config = MqttServerConfig {
    bind_addr: "0.0.0.0:1883".into(),
    namespace: "/mqtt".into(),      // MQTT topic "sensors/temp" -> CLASP "/mqtt/sensors/temp"
    require_auth: false,
    max_clients: 100,
    session_timeout_secs: 300,
    ..Default::default()
};
```

MQTT to CLASP mapping:

| MQTT | CLASP |
|------|-------|
| CONNECT | Hello -> Session |
| SUBSCRIBE `sensors/#` | Subscribe `/mqtt/sensors/**` |
| PUBLISH `sensors/temp` | Set `/mqtt/sensors/temp` |
| QoS 0 | Fire-and-forget |
| QoS 1 | With acknowledgment |

### OSC Server Adapter

Accept OSC clients via UDP with automatic session tracking:

```rust
use clasp_router::OscServerConfig;

let osc_config = OscServerConfig {
    bind_addr: "0.0.0.0:8000".into(),
    namespace: "/osc".into(),       // OSC "/synth/volume" -> CLASP "/osc/synth/volume"
    session_timeout_secs: 30,       // Sessions expire after 30s of inactivity
    auto_subscribe: false,
    ..Default::default()
};
```

## Configuration Reference

### RouterConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | String | "CLASP Router" | Server name shown to clients |
| `max_sessions` | usize | 1000 | Maximum concurrent connections |
| `session_timeout` | u64 | 300 | Session timeout in seconds |
| `security_mode` | SecurityMode | Open | Authentication mode |
| `max_subscriptions_per_session` | usize | 1000 | Max subscriptions per client |
| `gesture_coalescing` | bool | true | Enable gesture move coalescing |
| `gesture_coalesce_interval_ms` | u64 | 16 | Coalesce interval (16ms = 60fps) |
| `max_messages_per_second` | u32 | 1000 | Rate limit per client (0 = unlimited) |
| `rate_limiting_enabled` | bool | true | Enable rate limiting |

### Rate Limiting

Rate limiting prevents clients from overwhelming the router:

```rust
let config = RouterConfig {
    rate_limiting_enabled: true,
    max_messages_per_second: 500,  // 500 msg/s per client
    ..Default::default()
};
```

When a client exceeds the rate limit, excess messages are dropped and a warning is logged.

## Architecture

```
                    ┌─────────────────────────────────────────┐
                    │              CLASP Router               │
                    │  ┌─────────────────────────────────────┐│
                    │  │            Shared State             ││
                    │  │  sessions | subscriptions | state   ││
                    │  └─────────────────────────────────────┘│
                    │        ▲           ▲           ▲        │
                    │        │           │           │        │
                    │  ┌─────┴───┐ ┌─────┴───┐ ┌─────┴───┐   │
                    │  │WebSocket│ │  MQTT   │ │   OSC   │   │
                    │  │ :7330   │ │  :1883  │ │  :8000  │   │
                    │  └─────────┘ └─────────┘ └─────────┘   │
                    └─────────────────────────────────────────┘
```

All protocol adapters share the same router state, enabling cross-protocol communication.

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
