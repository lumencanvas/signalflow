# clasp-transport

Transport layer implementations for CLASP (Creative Low-Latency Application Streaming Protocol).

## Supported Transports

- **WebSocket** - Primary transport for browser and server communication
- **QUIC** - Low-latency UDP-based transport (optional)
- **TCP** - Reliable streaming transport (optional)

## Usage

```rust
use clasp_transport::WebSocketTransport;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let transport = WebSocketTransport::connect("ws://localhost:7330").await?;

    // Send messages
    transport.send(message).await?;

    // Receive messages
    while let Some(msg) = transport.recv().await {
        println!("Received: {:?}", msg);
    }

    Ok(())
}
```

## Features

- Async/await with Tokio
- Automatic frame encoding/decoding
- Connection health monitoring
- TLS support

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
