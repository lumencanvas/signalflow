# Embed Router

Embed a CLASP router directly into your application.

## Overview

Instead of running a separate router process, you can embed the router in your application. This is useful for:

- Single-binary deployments
- Custom routing logic
- Tight integration with application state
- Reduced operational complexity

## Rust

### Basic Embedded Router

```rust
use clasp_router::{Router, RouterConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure router
    let config = RouterConfig {
        name: "Embedded Router".into(),
        max_sessions: 100,
        ..Default::default()
    };

    // Create router (not async)
    let router = Router::new(config);

    // Clone for background task
    let router_clone = router.clone();

    // Run router in background
    let router_handle = tokio::spawn(async move {
        router_clone.serve_websocket("0.0.0.0:7330").await
    });

    // Your application logic here
    println!("Application running with embedded router");

    // Access state directly
    let state = router.state();
    println!("State entries: {}", state.len());

    // Wait for router (or handle shutdown)
    router_handle.await??;
    Ok(())
}
```

### Direct State Access

Access router state without network overhead:

```rust
use clasp_router::{Router, RouterConfig};
use clasp_core::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new(RouterConfig::default());

    // Clone for background task
    let router_clone = router.clone();
    tokio::spawn(async move {
        router_clone.serve_websocket("0.0.0.0:7330").await
    });

    // Access state directly (no network)
    let state = router.state();

    // Read values
    if let Some(value) = state.get("/sensors/temp") {
        println!("Temperature: {:?}", value);
    }

    // Get with metadata
    if let Some(param_state) = state.get_state("/sensors/temp") {
        println!("Value: {:?}, Revision: {}", param_state.value, param_state.revision);
    }

    // Get all matching a pattern
    let matches = state.get_matching("/sensors/**");
    for (address, param_state) in matches {
        println!("{}: {:?}", address, param_state.value);
    }

    Ok(())
}
```

### Token Validation

Add authentication to your embedded router:

```rust
use clasp_router::{Router, RouterConfig, CpskValidator};

let validator = CpskValidator::new("your-pre-shared-key");

let router = Router::new(RouterConfig::default())
    .with_validator(validator);

router.serve_websocket("0.0.0.0:7330").await?;
```

## JavaScript (Node.js)

Note: The JavaScript router package (`@clasp-to/router`) is separate from the client package.

```javascript
const { ClaspBuilder } = require('@clasp-to/core');

// For embedded routing in Node.js, you typically run the Rust router
// as a subprocess or use the client to connect to an external router.

// Connect as a client
const client = await new ClaspBuilder('ws://localhost:7330')
  .name('Node App')
  .connect();

// Use the client
await client.set('/app/status', 'running');

client.on('/control/**', (value, address) => {
  console.log(`${address} = ${value}`);
});
```

## Configuration Options

### Using RouterConfig

```rust
use clasp_router::RouterConfig;
use clasp_core::SecurityMode;

let config = RouterConfig {
    name: "My Router".into(),
    max_sessions: 1000,
    session_timeout: 300,
    security_mode: SecurityMode::Open,
    max_subscriptions_per_session: 1000,
    gesture_coalescing: true,
    gesture_coalesce_interval_ms: 16,
    rate_limiting_enabled: true,
    max_messages_per_second: 500,
    ..Default::default()
};
```

### Using RouterConfigBuilder

```rust
use clasp_router::RouterConfigBuilder;

let config = RouterConfigBuilder::new()
    .name("My Router")
    .max_sessions(1000)
    .session_timeout(300)
    .gesture_coalescing(true)
    .build();
```

## Graceful Shutdown

```rust
use tokio::signal;
use clasp_router::{Router, RouterConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new(RouterConfig::default());

    let router_for_server = router.clone();
    let server_handle = tokio::spawn(async move {
        router_for_server.serve_websocket("0.0.0.0:7330").await
    });

    // Wait for shutdown signal
    signal::ctrl_c().await?;

    // Stop the router
    router.stop();

    // Wait for server task to complete
    let _ = server_handle.await;

    Ok(())
}
```

## Multi-Protocol Router

Serve multiple protocols from a single embedded router:

```rust
use clasp_router::{Router, RouterConfig, MultiProtocolConfig, MqttServerConfig, OscServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RouterConfig {
        name: "My Multi-Protocol Router".into(),
        rate_limiting_enabled: true,
        max_messages_per_second: 500,
        ..Default::default()
    };

    let router = Router::new(config);

    let protocol_config = MultiProtocolConfig {
        websocket_addr: Some("0.0.0.0:7330".into()),
        mqtt: Some(MqttServerConfig {
            bind_addr: "0.0.0.0:1883".into(),
            namespace: "/mqtt".into(),
            ..Default::default()
        }),
        osc: Some(OscServerConfig {
            bind_addr: "0.0.0.0:8000".into(),
            namespace: "/osc".into(),
            ..Default::default()
        }),
        ..Default::default()
    };

    // All protocols share the same state
    router.serve_all(protocol_config).await?;
    Ok(())
}
```

This allows:
- MQTT clients on port 1883 (topics prefixed with `/mqtt/`)
- OSC clients on port 8000 (addresses prefixed with `/osc/`)
- WebSocket clients on port 7330 (native CLASP protocol)
- All protocols share state and can communicate cross-protocol

## Router Statistics

```rust
let router = Router::new(RouterConfig::default());

// Get current counts
println!("Sessions: {}", router.session_count());
println!("Subscriptions: {}", router.subscription_count());
println!("Active gestures: {}", router.active_gesture_count());
println!("State entries: {}", router.state().len());
```

## Performance Considerations

### Direct State Access

Accessing `router.state()` directly bypasses the network stack entirely:

```rust
// Network path: client -> serialize -> network -> deserialize -> router
// Direct path: router.state().get() -> immediate memory access

let state = router.state();
let value = state.get("/sensors/temp");  // Direct memory access
```

### Shared State Between Protocols

When using `serve_all()`, all protocols share the same state:

```rust
// MQTT client publishes to "sensors/temp"
// -> Stored at /mqtt/sensors/temp
// -> WebSocket client subscribed to /mqtt/** receives it
// -> OSC client can also subscribe to /mqtt/sensors/**
```

## Next Steps

- [Performance Tuning](performance-tuning.md)
- [Router Reference](../../reference/api/rust/clasp-router.md)
- [Start Router](../connections/start-router.md)
