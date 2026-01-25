# clasp-client (Rust)

CLASP client library for Rust applications.

## Overview

`clasp-client` provides an async client for connecting to CLASP routers.

```toml
[dependencies]
clasp-client = "3.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use clasp_client::{Clasp, ClaspBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("My App")
        .connect()
        .await?;

    // Set a value
    client.set("/sensors/temp", 23.5).await?;

    // Get a value
    let value = client.get("/sensors/temp").await?;
    println!("Temperature: {:?}", value);

    // Subscribe to changes
    client.on("/sensors/**", |value, address| async move {
        println!("{}: {:?}", address, value);
    }).await?;

    Ok(())
}
```

## Connection Methods

### Using ClaspBuilder

```rust
use clasp_client::ClaspBuilder;

let client = ClaspBuilder::new("ws://localhost:7330")
    .name("my-rust-client")
    .connect()
    .await?;
```

### Using Clasp::connect_to

```rust
use clasp_client::Clasp;

let client = Clasp::connect_to("ws://localhost:7330").await?;
```

### Using Clasp::builder

```rust
use clasp_client::Clasp;

let client = Clasp::builder("ws://localhost:7330")
    .name("my-client")
    .token("eyJhbGciOi...")
    .connect()
    .await?;
```

### With TLS

```rust
use clasp_client::ClaspBuilder;

// TLS is automatic when using wss:// URLs
let client = ClaspBuilder::new("wss://localhost:7330")
    .name("My App")
    .connect()
    .await?;
```

### With Auto-Reconnect

```rust
use clasp_client::ClaspBuilder;

let client = ClaspBuilder::new("ws://localhost:7330")
    .name("my-client")
    .reconnect(true)
    .reconnect_interval(5000)  // 5 seconds
    .connect()
    .await?;
```

## Core Operations

### Set

```rust
// Set with automatic type conversion
client.set("/path/to/value", 42).await?;
client.set("/path/to/value", "hello").await?;
client.set("/path/to/value", true).await?;

// Set with explicit Value
use clasp_core::Value;
client.set("/path/to/value", Value::Float(3.14)).await?;
```

### Get

```rust
// Get raw value
let value: Value = client.get("/path/to/value").await?;
println!("Value: {:?}", value);
```

### Emit (Events)

```rust
use clasp_core::Value;

// Emit event with payload
client.emit("/events/button_pressed", Value::Int(1)).await?;

// Emit without payload
client.emit("/events/ping", Value::Null).await?;
```

### Stream (High-Rate Data)

```rust
// Send continuous data
client.stream("/audio/level", 0.75).await?;
```

## Subscriptions

### Subscribe to Pattern

```rust
// Subscribe with async closure
client.on("/sensors/**", |value, address| async move {
    println!("{}: {:?}", address, value);
}).await?;
```

### Subscribe (Alias)

```rust
// subscribe() is an alias for on()
client.subscribe("/sensors/**", |value, address| async move {
    println!("{}: {:?}", address, value);
}).await?;
```

### Unsubscribe

```rust
// Store subscription ID
let sub_id = client.on("/sensors/**", handler).await?;

// Unsubscribe when done
client.unsubscribe(sub_id).await?;
```

## Gestures

For phased interactions (touch, drag, etc.):

```rust
use clasp_core::{Value, GesturePhase};

// Begin gesture
client.gesture(
    "/draw/stroke",
    GesturePhase::Begin,
    Value::Map(vec![
        ("x".into(), Value::Float(100.0)),
        ("y".into(), Value::Float(100.0)),
    ].into_iter().collect())
).await?;

// Update during gesture
client.gesture(
    "/draw/stroke",
    GesturePhase::Update,
    Value::Map(vec![
        ("x".into(), Value::Float(150.0)),
        ("y".into(), Value::Float(120.0)),
    ].into_iter().collect())
).await?;

// End gesture
client.gesture(
    "/draw/stroke",
    GesturePhase::End,
    Value::Map(vec![
        ("x".into(), Value::Float(200.0)),
        ("y".into(), Value::Float(150.0)),
    ].into_iter().collect())
).await?;
```

## Bundles

Send multiple messages atomically:

```rust
use clasp_core::Message;

let messages = vec![
    Message::set("/lights/1", Value::Int(255)),
    Message::set("/lights/2", Value::Int(128)),
];

client.bundle(messages).await?;
```

### Scheduled Bundle

```rust
let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)?
    .as_millis() as u64 + 5000;  // 5 seconds from now

client.bundle_at(messages, timestamp).await?;
```

## Connection State

```rust
// Check if connected
if client.is_connected() {
    println!("Connected!");
}

// Get session ID
if let Some(session_id) = client.session_id() {
    println!("Session: {}", session_id);
}

// Get synchronized time
let time = client.time();
```

## Cached Values

Access locally cached state:

```rust
// Get cached value (no network request)
if let Some(value) = client.cached("/sensors/temp") {
    println!("Cached: {:?}", value);
}
```

## Signal Discovery

```rust
// Get all announced signals
let signals = client.signals();
for signal in signals {
    println!("{}: {:?}", signal.address, signal.signal_type);
}

// Query signals matching pattern
let temp_signals = client.query_signals("/sensors/**");
```

## Error Handling

```rust
use clasp_client::ClientError;

match client.get("/path").await {
    Ok(value) => println!("{:?}", value),
    Err(ClientError::NotConnected) => println!("Not connected"),
    Err(ClientError::Timeout) => println!("Request timed out"),
    Err(e) => println!("Error: {:?}", e),
}

// Get last error
if let Some(error) = client.last_error() {
    println!("Last error: {:?}", error);
}

// Clear error state
client.clear_error();
```

## Graceful Shutdown

```rust
// Close connection gracefully
client.close().await;
```

## See Also

- [clasp-core](clasp-core.md) - Core types
- [clasp-router](clasp-router.md) - Router library
- [Connect Client](../../../how-to/connections/connect-client.md)
