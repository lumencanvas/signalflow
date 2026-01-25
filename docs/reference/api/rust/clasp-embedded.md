# clasp-embedded (Rust)

No-std CLASP client for embedded systems.

## Overview

`clasp-embedded` provides a minimal CLASP client for microcontrollers and embedded systems with limited resources.

```toml
[dependencies]
clasp-embedded = { version = "3.1", default-features = false }
```

## Features

- **No heap allocation** - All buffers are stack-allocated
- **No-std compatible** - Works without standard library
- **Minimal footprint** - ~3.6KB RAM usage
- **Transport agnostic** - Bring your own network stack

## Quick Start

```rust
#![no_std]
#![no_main]

use clasp_embedded::{Client, Value, Frame};

#[entry]
fn main() -> ! {
    let mut clasp = Client::new();

    loop {
        let temp = read_temperature();

        // Prepare SET message
        let frame = clasp.prepare_set(
            "/sensors/device1/temp",
            Value::Float(temp as f64)
        );

        // Send via your transport
        uart_send(&frame);

        delay_ms(5000);
    }
}
```

## Client

### Initialization

```rust
use clasp_embedded::Client;

// Stack-allocated client
let mut client = Client::new();

// With custom buffer size
let mut client = Client::<256>::new();  // 256-byte message buffer
```

### Configuration

```rust
use clasp_embedded::{Client, ClientConfig};

let config = ClientConfig {
    client_id: 0x1234,
    max_message_size: 128,
};

let mut client = Client::with_config(config);
```

## Creating Messages

### SET Message

```rust
use clasp_embedded::{Client, Value};

let mut client = Client::new();

// Simple values
let frame = client.prepare_set("/path", Value::Int(42));
let frame = client.prepare_set("/path", Value::Float(3.14));
let frame = client.prepare_set("/path", Value::Bool(true));

// With pre-allocated buffer
let mut buffer = [0u8; 64];
let len = client.encode_set("/path", Value::Int(42), &mut buffer)?;
// buffer[..len] contains the encoded message
```

### EMIT Message

```rust
let frame = client.prepare_emit("/event", Value::Null);
```

### GET Message

```rust
let (frame, request_id) = client.prepare_get("/path");
// Store request_id to match with response
```

## Values

### Value Type

```rust
use clasp_embedded::Value;

pub enum Value<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(&'a str),
    Bytes(&'a [u8]),
}

// Create values
let v = Value::Int(42);
let v = Value::Float(23.5);
let v = Value::String("hello");
let v = Value::Bytes(&[0x01, 0x02, 0x03]);
```

### Fixed-Point Values

For systems without floating-point:

```rust
use clasp_embedded::FixedValue;

// 16.16 fixed point
let v = FixedValue::from_fixed(1572864);  // Represents 24.0
let v = FixedValue::from_int_frac(24, 0);
```

## Parsing Responses

### Parse Incoming Message

```rust
use clasp_embedded::{parse_message, Message};

fn handle_incoming(data: &[u8]) {
    match parse_message(data) {
        Ok(Message::Set { address, value, .. }) => {
            if address == "/config/brightness" {
                if let Value::Int(level) = value {
                    set_led_brightness(level as u8);
                }
            }
        }
        Ok(Message::Ack { id }) => {
            // Message confirmed
        }
        Ok(Message::Error { id, code, message }) => {
            // Handle error
        }
        Err(e) => {
            // Parse error
        }
        _ => {}
    }
}
```

### Streaming Parser

For handling partial data:

```rust
use clasp_embedded::StreamParser;

let mut parser = StreamParser::new();

fn on_uart_byte(byte: u8) {
    if let Some(message) = parser.feed(byte) {
        handle_message(message);
    }
}
```

## Frame Format

### Binary Frame

```rust
use clasp_embedded::Frame;

pub struct Frame<'a> {
    pub data: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn message_type(&self) -> u8;
    pub fn payload(&self) -> &[u8];
    pub fn len(&self) -> usize;
}
```

### Manual Frame Construction

```rust
use clasp_embedded::frame::{FrameBuilder, MessageType};

let mut buffer = [0u8; 128];
let mut builder = FrameBuilder::new(&mut buffer);

builder.set_type(MessageType::Set);
builder.write_address("/sensors/temp")?;
builder.write_value(Value::Float(23.5))?;

let frame = builder.finish()?;
```

## Transport Integration

### UART Example

```rust
use clasp_embedded::Client;

fn send_to_router(frame: &[u8]) {
    for byte in frame {
        uart_write(*byte);
    }
}

fn main_loop() {
    let mut client = Client::new();
    let mut rx_buffer = [0u8; 128];
    let mut rx_pos = 0;

    loop {
        // Send sensor data
        let frame = client.prepare_set("/sensor", Value::Float(read_temp()));
        send_to_router(frame.as_bytes());

        // Receive responses
        while let Some(byte) = uart_read() {
            rx_buffer[rx_pos] = byte;
            rx_pos += 1;

            if let Ok(msg) = parse_message(&rx_buffer[..rx_pos]) {
                handle_message(msg);
                rx_pos = 0;
            }
        }

        delay_ms(100);
    }
}
```

### SPI Example

```rust
fn send_frame_spi(frame: &[u8]) {
    cs_low();
    for byte in frame {
        spi_transfer(*byte);
    }
    cs_high();
}
```

### WiFi (ESP32) Example

```rust
use clasp_embedded::Client;
use esp_wifi::wifi::WifiStack;

fn send_to_router(socket: &mut TcpSocket, frame: &[u8]) {
    socket.write_all(frame).ok();
}
```

## Memory Management

### Static Buffers

```rust
// Define static buffers
static mut TX_BUFFER: [u8; 256] = [0u8; 256];
static mut RX_BUFFER: [u8; 256] = [0u8; 256];

fn init() {
    let client = Client::with_buffers(
        unsafe { &mut TX_BUFFER },
        unsafe { &mut RX_BUFFER }
    );
}
```

### Buffer Pools

```rust
use clasp_embedded::BufferPool;

// Pool of 4 x 64-byte buffers
static POOL: BufferPool<4, 64> = BufferPool::new();

fn send_message() {
    let mut buf = POOL.alloc().unwrap();
    // Use buffer
    // Automatically returned to pool on drop
}
```

## Platform Support

### Cortex-M

```toml
[dependencies]
clasp-embedded = { version = "3.1", features = ["cortex-m"] }
```

### RISC-V

```toml
[dependencies]
clasp-embedded = { version = "3.1", features = ["riscv"] }
```

### ESP32

```toml
[dependencies]
clasp-embedded = { version = "3.1", features = ["esp32"] }
```

## Error Handling

```rust
use clasp_embedded::Error;

pub enum Error {
    BufferTooSmall,
    InvalidMessage,
    InvalidAddress,
    InvalidValue,
    EncodingError,
}

// No-panic error handling
fn try_send() -> Result<(), Error> {
    let frame = client.prepare_set("/path", value)?;
    send(frame.as_bytes())?;
    Ok(())
}
```

## Size Optimization

```toml
# Cargo.toml for minimal size
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"

[dependencies]
clasp-embedded = { version = "3.1", default-features = false }
```

Typical sizes:
- Code: ~8KB
- RAM: ~3.6KB (with 256-byte buffers)

## See Also

- [Embedded Systems Guide](../../../use-cases/embedded-systems.md)
- [Embedded Sensor Tutorial](../../../tutorials/embedded-sensor-node.md)
- [clasp-core](clasp-core.md) - Full-featured core types
