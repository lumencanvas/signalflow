# clasp-core

Core types and encoding for the CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **Message Types**: Set, Publish, Subscribe, Bundle, Snapshot, etc.
- **Value Types**: Int, Float, Bool, String, Bytes, Array, Map, Null
- **Binary Encoding (v3)**: 55% smaller, 4x faster than JSON/MessagePack
- **Address Patterns**: Hierarchical addressing with wildcards (`*`, `**`)
- **Signal Types**: Param, Event, Stream, Gesture, Timeline

## Usage

```rust
use clasp_core::{Message, SetMessage, Value, codec};

// Create a set message
let msg = Message::Set(SetMessage {
    address: "/lights/front/brightness".to_string(),
    value: Value::Float(0.75),
    revision: None,
    lock: false,
    unlock: false,
});

// Encode to v3 binary format
let encoded = codec::encode(&msg).unwrap();

// Decode (auto-detects v2/v3)
let (decoded, _frame) = codec::decode(&encoded).unwrap();
```

## Binary Encoding

CLASP binary encoding is 55% smaller and 4-7x faster than JSON/MessagePack:

| Metric | JSON | CLASP Binary |
|--------|------|--------------|
| SET size | ~80 bytes | 31 bytes |
| Encode | ~2M msg/s | 8M msg/s |
| Decode | ~2M msg/s | 11M msg/s |

## Address Patterns

CLASP uses hierarchical addresses with wildcard support:

| Pattern | Matches |
|---------|---------|
| `/lights/front` | Exact match |
| `/lights/*` | Single segment wildcard |
| `/lights/**` | Multi-segment wildcard |
| `/lights/zone5*` | Embedded wildcard |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio)
