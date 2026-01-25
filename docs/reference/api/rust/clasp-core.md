# clasp-core (Rust)

Core types and codec for the CLASP protocol.

## Overview

`clasp-core` provides the foundational types and binary encoding/decoding for CLASP messages. It's used by all other CLASP Rust crates.

```toml
[dependencies]
clasp-core = "3.1"
```

## Features

```toml
# Default features
clasp-core = "3.1"

# No-std support (embedded)
clasp-core = { version = "3.1", default-features = false }

# With serde serialization
clasp-core = { version = "3.1", features = ["serde"] }
```

## Message Types

### Message

The main message enum:

```rust
use clasp_core::{Message, MessageType};

pub enum Message {
    Hello(HelloMessage),
    Set(SetMessage),
    Get(GetMessage),
    Subscribe(SubscribeMessage),
    Unsubscribe(UnsubscribeMessage),
    Emit(EmitMessage),
    Lock(LockMessage),
    Unlock(UnlockMessage),
    Bundle(BundleMessage),
    Ack(AckMessage),
    Error(ErrorMessage),
}

impl Message {
    pub fn message_type(&self) -> MessageType;
    pub fn id(&self) -> Option<u32>;
    pub fn address(&self) -> Option<&str>;
}
```

### SetMessage

```rust
use clasp_core::{SetMessage, Value, SignalType, QoS};

pub struct SetMessage {
    pub id: Option<u32>,
    pub address: String,
    pub value: Value,
    pub signal_type: SignalType,
    pub qos: QoS,
    pub timestamp: Option<u64>,
}

// Create a SET message
let msg = SetMessage {
    id: Some(1),
    address: "/sensors/temp".into(),
    value: Value::Float(23.5),
    signal_type: SignalType::Param,
    qos: QoS::Fire,
    timestamp: None,
};
```

### GetMessage

```rust
pub struct GetMessage {
    pub id: u32,
    pub address: String,
}

let msg = GetMessage {
    id: 1,
    address: "/sensors/temp".into(),
};
```

### SubscribeMessage

```rust
pub struct SubscribeMessage {
    pub id: u32,
    pub pattern: String,
    pub options: SubscribeOptions,
}

pub struct SubscribeOptions {
    pub max_rate: Option<f64>,
    pub debounce: Option<u64>,
    pub include_initial: bool,
}
```

### BundleMessage

```rust
pub struct BundleMessage {
    pub id: Option<u32>,
    pub messages: Vec<Message>,
    pub timestamp: Option<u64>,
    pub qos: QoS,
}
```

## Value Types

### Value Enum

```rust
use clasp_core::Value;

pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Blob(Vec<u8>),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
}

impl Value {
    // Type checking
    pub fn is_null(&self) -> bool;
    pub fn is_bool(&self) -> bool;
    pub fn is_number(&self) -> bool;
    pub fn is_string(&self) -> bool;

    // Conversion
    pub fn as_bool(&self) -> Option<bool>;
    pub fn as_i64(&self) -> Option<i64>;
    pub fn as_f64(&self) -> Option<f64>;
    pub fn as_str(&self) -> Option<&str>;
    pub fn as_bytes(&self) -> Option<&[u8]>;
    pub fn as_array(&self) -> Option<&Vec<Value>>;
    pub fn as_map(&self) -> Option<&HashMap<String, Value>>;
}
```

### Value Conversion

```rust
use clasp_core::Value;

// From primitives
let v: Value = 42.into();
let v: Value = 3.14.into();
let v: Value = "hello".into();
let v: Value = true.into();

// From collections
let v: Value = vec![1, 2, 3].into();
let v: Value = vec![("key", "value")].into();

// With serde feature
#[derive(Serialize, Deserialize)]
struct SensorData {
    temp: f64,
    humidity: f64,
}

let data = SensorData { temp: 23.5, humidity: 65.0 };
let v: Value = serde_json::to_value(&data)?.into();
```

## Signal Types

```rust
use clasp_core::SignalType;

pub enum SignalType {
    Param,      // Stateful, retained value
    Event,      // Ephemeral, one-time
    Stream,     // High-rate continuous data
    Gesture,    // Phased interaction (begin/update/end)
    Timeline,   // Time-indexed automation
}
```

## QoS Levels

```rust
use clasp_core::QoS;

pub enum QoS {
    Fire,       // Best effort, no confirmation
    Confirm,    // Acknowledged delivery
    Commit,     // Ordered, exactly-once
}
```

## Codec

### Encoding

```rust
use clasp_core::{Codec, Message, SetMessage, Value};

let codec = Codec::new();

let msg = Message::Set(SetMessage {
    id: Some(1),
    address: "/test".into(),
    value: Value::Int(42),
    signal_type: SignalType::Param,
    qos: QoS::Fire,
    timestamp: None,
});

// Encode to bytes
let bytes: Vec<u8> = codec.encode(&msg)?;
```

### Decoding

```rust
use clasp_core::{Codec, Message};

let codec = Codec::new();

// Decode from bytes
let msg: Message = codec.decode(&bytes)?;

match msg {
    Message::Set(set) => {
        println!("SET {} = {:?}", set.address, set.value);
    }
    Message::Get(get) => {
        println!("GET {}", get.address);
    }
    _ => {}
}
```

## Address Matching

```rust
use clasp_core::address::{matches, parse_pattern};

// Simple wildcard
assert!(matches("/sensors/temp", "/sensors/*"));

// Multi-level wildcard
assert!(matches("/sensors/room1/temp", "/sensors/**"));

// Parse pattern for efficient reuse
let pattern = parse_pattern("/sensors/*/temp")?;
assert!(pattern.matches("/sensors/room1/temp"));
assert!(pattern.matches("/sensors/room2/temp"));
```

## Frame Format

The binary frame structure:

```rust
use clasp_core::frame::{Frame, FrameHeader};

pub struct Frame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

pub struct FrameHeader {
    pub version: u8,
    pub message_type: u8,
    pub flags: u8,
    pub length: u32,
}

// Low-level frame operations
let frame = Frame::from_message(&msg)?;
let bytes = frame.to_bytes();
let frame = Frame::from_bytes(&bytes)?;
```

## Error Types

```rust
use clasp_core::Error;

pub enum Error {
    InvalidMessage(String),
    EncodingError(String),
    DecodingError(String),
    InvalidAddress(String),
    InvalidValue(String),
}

impl std::error::Error for Error {}
```

## No-std Support

For embedded systems:

```rust
#![no_std]

use clasp_core::{Message, Value, Codec};

// All core types work without std
let value = Value::Float(23.5);
let codec = Codec::new();
```

## Thread Safety

All types in `clasp-core` are `Send + Sync`:

```rust
use std::sync::Arc;
use clasp_core::{Message, Codec};

let codec = Arc::new(Codec::new());

// Safe to share across threads
let codec_clone = codec.clone();
std::thread::spawn(move || {
    let msg = codec_clone.decode(&bytes).unwrap();
});
```

## See Also

- [clasp-client](clasp-client.md) - Client library
- [clasp-router](clasp-router.md) - Router library
- [Protocol Reference](../../protocol/overview.md)
