# clasp-core

Core types and encoding for the CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **Message Types**: Set, Get, Subscribe, Unsubscribe, Batch operations
- **Value Types**: Int, Float, Bool, String, Bytes, Array, Map, Null
- **Encoding**: MessagePack (binary) and JSON serialization
- **Address Patterns**: Hierarchical addressing with wildcards (`*`, `**`)
- **no_std Support**: Optional `alloc` and `std` features

## Usage

```rust
use clasp_core::{Message, SetMessage, Value};

// Create a set message
let msg = Message::Set(SetMessage {
    address: "/lights/front/brightness".to_string(),
    value: Value::Float(0.75),
    revision: None,
    lock: false,
    unlock: false,
});

// Encode to MessagePack
let encoded = clasp_core::encode(&msg).unwrap();

// Decode from MessagePack
let decoded: Message = clasp_core::decode(&encoded).unwrap();
```

## Address Patterns

CLASP uses hierarchical addresses with wildcard support:

| Pattern | Matches |
|---------|---------|
| `/lights/front` | Exact match |
| `/lights/*` | Single segment wildcard |
| `/lights/**` | Multi-segment wildcard |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
