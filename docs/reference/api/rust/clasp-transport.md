# clasp-transport (Rust)

Transport layer implementations for CLASP.

## Overview

`clasp-transport` provides various transport mechanisms for CLASP communication.

```toml
[dependencies]
clasp-transport = "3.1"

# Or select specific transports
clasp-transport = { version = "3.1", features = ["websocket", "quic"] }
```

## Features

```toml
# All transports
clasp-transport = { version = "3.1", features = ["full"] }

# Individual transports
clasp-transport = { version = "3.1", features = [
    "websocket",
    "quic",
    "udp",
    "tcp",
    "webrtc",
    "serial",
    "ble"
] }
```

## Transport Trait

```rust
use clasp_transport::{Transport, TransportConfig};
use async_trait::async_trait;

#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to a remote endpoint
    async fn connect(&mut self) -> Result<()>;

    /// Send a message
    async fn send(&self, data: &[u8]) -> Result<()>;

    /// Receive a message
    async fn recv(&self) -> Result<Vec<u8>>;

    /// Close the connection
    async fn close(&self) -> Result<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;
}
```

## WebSocket Transport

### Client

```rust
use clasp_transport::websocket::{WsTransport, WsConfig};

let config = WsConfig {
    url: "ws://localhost:7330".parse()?,
    ..Default::default()
};

let mut transport = WsTransport::new(config);
transport.connect().await?;

// Send message
transport.send(b"hello").await?;

// Receive message
let data = transport.recv().await?;
```

### With TLS

```rust
let config = WsConfig {
    url: "wss://localhost:7330".parse()?,
    tls_config: Some(TlsConfig {
        cert_path: Some("/path/to/cert.pem".into()),
        accept_invalid_certs: false,
    }),
    ..Default::default()
};
```

### Server

```rust
use clasp_transport::websocket::WsListener;

let listener = WsListener::bind("0.0.0.0:7330").await?;

loop {
    let (transport, addr) = listener.accept().await?;
    tokio::spawn(async move {
        handle_client(transport).await
    });
}
```

## QUIC Transport

### Client

```rust
use clasp_transport::quic::{QuicTransport, QuicConfig};

let config = QuicConfig {
    server_addr: "localhost:7330".parse()?,
    server_name: "localhost".into(),
    cert_path: Some("/path/to/cert.pem".into()),
    ..Default::default()
};

let mut transport = QuicTransport::new(config);
transport.connect().await?;
```

### Server

```rust
use clasp_transport::quic::QuicListener;

let listener = QuicListener::builder()
    .bind("0.0.0.0:7330")
    .cert_path("/path/to/cert.pem")
    .key_path("/path/to/key.pem")
    .build()
    .await?;

loop {
    let transport = listener.accept().await?;
    tokio::spawn(handle_client(transport));
}
```

### Streams

QUIC supports multiple streams:

```rust
// Open new stream
let stream_id = transport.open_stream().await?;

// Send on specific stream
transport.send_on_stream(stream_id, data).await?;

// Receive from stream
let data = transport.recv_from_stream(stream_id).await?;
```

## UDP Transport

### Basic UDP

```rust
use clasp_transport::udp::{UdpTransport, UdpConfig};

let config = UdpConfig {
    bind_addr: "0.0.0.0:0".parse()?,
    target_addr: "192.168.1.100:7330".parse()?,
};

let transport = UdpTransport::new(config).await?;

// Send (unreliable)
transport.send(data).await?;

// Receive
let (data, from_addr) = transport.recv_from().await?;
```

### Multicast

```rust
let config = UdpConfig {
    bind_addr: "0.0.0.0:7330".parse()?,
    multicast_groups: vec!["239.255.0.1".parse()?],
    ..Default::default()
};

let transport = UdpTransport::new(config).await?;

// Send to multicast group
transport.send_to(data, "239.255.0.1:7330").await?;
```

## WebRTC Transport

### Peer Connection

```rust
use clasp_transport::webrtc::{WebRtcTransport, WebRtcConfig, IceServer};

let config = WebRtcConfig {
    ice_servers: vec![
        IceServer {
            urls: vec!["stun:stun.l.google.com:19302".into()],
            ..Default::default()
        }
    ],
    ..Default::default()
};

let transport = WebRtcTransport::new(config).await?;

// Create offer
let offer = transport.create_offer().await?;

// Set remote answer
transport.set_remote_answer(answer).await?;

// Wait for connection
transport.wait_connected().await?;
```

### Data Channels

```rust
// Create data channel
let channel = transport.create_data_channel("control", DataChannelConfig {
    ordered: true,
    max_retransmits: None,
}).await?;

// Send on channel
channel.send(data).await?;

// Receive
let data = channel.recv().await?;
```

## Serial Transport

### Basic Serial

```rust
use clasp_transport::serial::{SerialTransport, SerialConfig};

let config = SerialConfig {
    port: "/dev/ttyUSB0".into(),
    baud_rate: 115200,
    data_bits: 8,
    stop_bits: 1,
    parity: Parity::None,
};

let mut transport = SerialTransport::new(config)?;
transport.connect().await?;
```

### Frame Delimiters

```rust
let config = SerialConfig {
    port: "/dev/ttyUSB0".into(),
    baud_rate: 115200,
    framing: Framing::LineDelimited('\n'),  // Or COBS, Length-prefixed, etc.
    ..Default::default()
};
```

## BLE Transport

### Central (Client)

```rust
use clasp_transport::ble::{BleTransport, BleConfig};

let config = BleConfig {
    device_name: Some("CLASP Device".into()),
    service_uuid: CLASP_SERVICE_UUID,
    characteristic_uuid: CLASP_CHAR_UUID,
};

let transport = BleTransport::new(config).await?;

// Scan and connect
transport.scan_and_connect().await?;
```

### Peripheral (Server)

```rust
use clasp_transport::ble::BlePeripheral;

let peripheral = BlePeripheral::builder()
    .name("CLASP Router")
    .service(CLASP_SERVICE_UUID)
    .characteristic(CLASP_CHAR_UUID)
    .build()
    .await?;

peripheral.start_advertising().await?;

loop {
    let transport = peripheral.accept().await?;
    tokio::spawn(handle_client(transport));
}
```

## Multiplexing

Layer multiple logical connections over one transport:

```rust
use clasp_transport::mux::{Multiplexer, MuxConfig};

let mux = Multiplexer::new(transport, MuxConfig::default());

// Create virtual streams
let stream1 = mux.open_stream().await?;
let stream2 = mux.open_stream().await?;

// Each stream acts like independent transport
stream1.send(data1).await?;
stream2.send(data2).await?;
```

## Connection Pooling

```rust
use clasp_transport::pool::{TransportPool, PoolConfig};

let pool = TransportPool::new(PoolConfig {
    max_connections: 10,
    idle_timeout: Duration::from_secs(60),
    ..Default::default()
});

// Get transport from pool
let transport = pool.get("ws://localhost:7330").await?;

// Use transport
transport.send(data).await?;

// Return to pool
pool.release(transport).await;
```

## Error Handling

```rust
use clasp_transport::Error;

match transport.connect().await {
    Ok(()) => println!("Connected"),
    Err(Error::ConnectionRefused) => println!("Connection refused"),
    Err(Error::Timeout) => println!("Connection timeout"),
    Err(Error::TlsError(e)) => println!("TLS error: {}", e),
    Err(Error::IoError(e)) => println!("I/O error: {}", e),
    Err(e) => println!("Error: {:?}", e),
}
```

## See Also

- [Transport Reference](../../transports/) - Transport-specific details
- [clasp-client](clasp-client.md) - Client library
- [P2P WebRTC](../../../how-to/advanced/p2p-webrtc.md)
