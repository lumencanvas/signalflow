# clasp-router (Rust)

CLASP router library for building routers and servers.

## Overview

`clasp-router` provides a high-performance router implementation with support for multiple protocols.

```toml
[dependencies]
clasp-router = "3.1"
tokio = { version = "1", features = ["full"] }

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

## Quick Start

```rust
use clasp_router::{Router, RouterConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RouterConfig::default();
    let router = Router::new(config);

    // Serve on WebSocket (blocks until error or shutdown)
    router.serve_websocket("0.0.0.0:7330").await?;

    Ok(())
}
```

## Configuration

### Using RouterConfig

```rust
use clasp_router::RouterConfig;
use clasp_core::SecurityMode;

let config = RouterConfig {
    name: "My Router".into(),
    max_sessions: 1000,
    session_timeout: 300,
    features: vec!["param".into(), "event".into()],
    security_mode: SecurityMode::Open,
    max_subscriptions_per_session: 1000,
    gesture_coalescing: true,
    gesture_coalesce_interval_ms: 16,
    rate_limiting_enabled: true,
    max_messages_per_second: 1000,
};

let router = Router::new(config);
```

### Using RouterConfigBuilder

```rust
use clasp_router::RouterConfigBuilder;

let config = RouterConfigBuilder::new()
    .name("My Router")
    .max_sessions(1000)
    .session_timeout(300)
    .gesture_coalescing(true)
    .gesture_coalesce_interval_ms(16)
    .build();

let router = Router::new(config);
```

### Rate Limiting

```rust
let config = RouterConfig {
    rate_limiting_enabled: true,
    max_messages_per_second: 500,  // Per client
    ..Default::default()
};
```

When a client exceeds the rate limit, excess messages are dropped and a warning is logged.

## Router Operations

### Serve on WebSocket

```rust
let router = Router::new(RouterConfig::default());

// Blocks until error or shutdown
router.serve_websocket("0.0.0.0:7330").await?;
```

### Serve on QUIC (with TLS)

```rust
let router = Router::new(RouterConfig::default());

router.serve_quic(
    "0.0.0.0:7331".parse()?,
    vec![cert_der],  // Certificate chain
    key_der,         // Private key
).await?;
```

### Run in Background

```rust
let router = Router::new(RouterConfig::default());

// Clone for the spawned task
let router_clone = router.clone();

tokio::spawn(async move {
    router_clone.serve_websocket("0.0.0.0:7330").await
});

// Continue with other work...
```

### Multi-Protocol Server

Serve multiple protocols simultaneously with shared state:

```rust
use clasp_router::{Router, RouterConfig, MultiProtocolConfig, MqttServerConfig, OscServerConfig};

let router = Router::new(RouterConfig::default());

let config = MultiProtocolConfig {
    websocket_addr: Some("0.0.0.0:7330".into()),
    mqtt: Some(MqttServerConfig {
        bind_addr: "0.0.0.0:1883".into(),
        namespace: "/mqtt".into(),
        require_auth: false,
        max_clients: 100,
        session_timeout_secs: 300,
        ..Default::default()
    }),
    osc: Some(OscServerConfig {
        bind_addr: "0.0.0.0:8000".into(),
        namespace: "/osc".into(),
        session_timeout_secs: 30,
        ..Default::default()
    }),
    ..Default::default()
};

// All protocols share the same router state
router.serve_all(config).await?;
```

Protocol adapters require feature flags:
- `mqtt-server` for MQTT support
- `osc-server` for OSC support

### Stop Router

```rust
// From another task or signal handler
router.stop();
```

## Direct State Access

Access router state directly:

```rust
let router = Router::new(RouterConfig::default());

// Get the state manager
let state = router.state();

// Read a value
if let Some(value) = state.get("/sensors/temp") {
    println!("Temperature: {:?}", value);
}

// Get with full metadata
if let Some(param_state) = state.get_state("/sensors/temp") {
    println!("Value: {:?}, Revision: {}", param_state.value, param_state.revision);
}

// Get all matching a pattern
let matches = state.get_matching("/sensors/**");
for (address, param_state) in matches {
    println!("{}: {:?}", address, param_state.value);
}

// Get a snapshot for late-joiner sync
let snapshot = state.snapshot("/sensors/**");
```

## Token Validation

### Built-in CPSK Validator

```rust
use clasp_router::CpskValidator;

let validator = CpskValidator::new("your-pre-shared-key");
let router = Router::new(RouterConfig::default())
    .with_validator(validator);
```

### Custom Validator

```rust
use clasp_router::TokenValidator;
use clasp_core::{Capabilities, SignalAccess};

struct MyValidator;

impl TokenValidator for MyValidator {
    fn validate(&self, token: &str) -> Option<Capabilities> {
        if token == "valid-token" {
            Some(Capabilities {
                read: vec![SignalAccess::Pattern("/**".into())],
                write: vec![SignalAccess::Pattern("/user/**".into())],
                ..Default::default()
            })
        } else {
            None
        }
    }
}

let router = Router::new(RouterConfig::default())
    .with_validator(MyValidator);
```

## Router Statistics

```rust
let router = Router::new(RouterConfig::default());

// Get current counts
println!("Sessions: {}", router.session_count());
println!("Subscriptions: {}", router.subscription_count());
println!("Active gestures: {}", router.active_gesture_count());
println!("State entries: {}", router.state().len());
```

## Error Handling

```rust
use clasp_router::RouterError;

match router.serve_websocket("0.0.0.0:7330").await {
    Ok(()) => println!("Router stopped normally"),
    Err(RouterError::Io(e)) => eprintln!("IO error: {}", e),
    Err(RouterError::Transport(e)) => eprintln!("Transport error: {}", e),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## See Also

- [clasp-core](clasp-core.md) - Core types
- [clasp-client](clasp-client.md) - Client library
- [Embed Router](../../../how-to/advanced/embed-router.md)
- [Start Router](../../../how-to/connections/start-router.md)
