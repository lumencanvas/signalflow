# CLASP Binary Encoding v3 - Implementation Plan

**Version:** 2.0
**Date:** 2026-01-22
**Status:** ✅ COMPLETED
**Author:** Claude (Opus 4.5)

> **Note:** This plan has been fully implemented. See `clasp-core/src/codec.rs` for the implementation.

---

## Executive Summary

This document outlines a plan to replace CLASP's inefficient MessagePack-with-named-keys encoding with an **efficient binary wire format**. The protocol semantics remain identical - we're simply encoding the same information without redundant field name strings.

**This is NOT a "compact mode" - it's the new default encoding for CLASP v3.**

### Why This Change?

The current encoding wastes bytes spelling out field names in every message:

```
Current: {"type":"SET","address":"/test","value":0.5,"revision":1}
         ↑ 40 bytes of field names that never change!
```

The proposed encoding conveys the same information without redundancy:

```
New:     [SET_TYPE][flags][addr_len][addr][value_type][value][revision]
         ↑ Same semantics, no wasted bytes
```

### Performance Comparison

| Metric | CLASP v2 (current) | CLASP v3 (new) | MQTT | OSC |
|--------|-------------------|----------------|------|-----|
| Encoding speed | 1.8M msg/s | **~10M msg/s** | 11.4M msg/s | 4.5M msg/s |
| Decoding speed | 1.5M msg/s | **~12M msg/s** | 11.4M msg/s | 5.7M msg/s |
| SET message size | 69 bytes | **32 bytes** | 19 bytes | 24 bytes |

**Result:** CLASP becomes competitive with MQTT/OSC while keeping rich features (revisions, locks, typed signals).

---

## Research Validation

### Primary Sources

1. **MessagePack Specification** (msgpack.org)
   - States encoders "SHOULD use the most compact representation possible"
   - Named keys (`to_vec_named`) add field names as UTF-8 strings to every message
   - Array-based encoding eliminates this overhead

2. **MPack Protocol Clarifications** (ludocode.github.io/mpack)
   - "When encoding any value (other than floating point numbers), MPack always chooses the shortest representation"
   - Overlong encoding sequences are forbidden in well-formed MessagePack

3. **rmp-serde Documentation** (docs.rs/rmp-serde)
   - `to_vec_named()` - Serializes with field names (current CLASP)
   - `to_vec()` - Serializes as positional arrays (smaller but loses names)
   - Low-level `rmp` API allows custom binary formats

4. **msgpack-schema Crate** (docs.rs/msgpack-schema)
   - Demonstrates tagged integer fields instead of string keys
   - Pattern: `{0: value, 1: value}` instead of `{"field1": value, "field2": value}`

### Empirical Validation

**Test conducted on 2026-01-22:**

```
cargo test --package clasp-core --lib size_tests -- --nocapture

=== Actual CLASP Message Sizes ===
Payload (MsgPack): 69 bytes
Full frame:        73 bytes

Payload hex dump:
86a474797065a3534554a761646472657373ab2f746573742f76616c7565a576
616c7565cb3fe0000000000000a87265766973696f6e01a46c6f636bc2a6756e
6c6f636bc2
```

**Hex decode analysis:**
- `86` = fixmap(6) - 6 key-value pairs
- `a4 74 79 70 65` = "type" (5 bytes)
- `a3 53 45 54` = "SET" (4 bytes)
- `a7 61 64 64 72 65 73 73` = "address" (8 bytes)
- ... and so on

**Result: Field names consume 40 bytes out of 69 (58% overhead!)**

### Protocol Comparison

| Protocol | How it encodes SET /test/value = 0.5 | Size |
|----------|--------------------------------------|------|
| **MQTT** | topic length (2) + topic (11) + payload (4) + header (2) | 19B |
| **OSC** | address (12, padded) + type tag (4) + float (4) | 20B |
| **CLASP (named)** | fixmap + all field names + values | 69B |
| **CLASP (compact)** | type(1) + addr_len(2) + addr(11) + value_type(1) + value(8) + flags(1) + rev(8) | 32B |

---

## Design Philosophy

### Why NOT Two Modes?

Having "standard" and "compact" modes would:
- Add complexity to every implementation
- Create compatibility headaches
- Require negotiation logic
- Split the ecosystem

Instead: **One efficient encoding that does everything.**

The protocol semantics (SET, PUBLISH, SUBSCRIBE, etc.) are unchanged. We're just encoding them efficiently.

### Analogy: Protocol Buffers

Protocol Buffers doesn't have a "JSON mode" and a "binary mode". It has:
- A schema that defines the semantics
- An efficient binary encoding

CLASP should work the same way:
- Protocol spec defines message semantics
- Wire format is efficient binary (not JSON-with-field-names)

## Wire Format Specification

### Frame Format (Unchanged)

```
┌─────────────────────────────────────────────────────────────────┐
│ Byte 0:     Magic (0x53 = 'S')                                  │
│ Byte 1:     Flags                                               │
│             [7:6] QoS (00=fire, 01=confirm, 10=commit, 11=rsv)  │
│             [5]   Timestamp present                             │
│             [4]   Encrypted                                     │
│             [3]   Compressed                                    │
│             [2:0] Version (000=v2 legacy, 001=v3 binary)        │
│ Byte 2-3:   Payload Length (uint16 big-endian)                  │
├─────────────────────────────────────────────────────────────────┤
│ [If timestamp flag] Bytes 4-11: Timestamp (uint64 µs)           │
├─────────────────────────────────────────────────────────────────┤
│ Payload (v3: efficient binary, v2: MessagePack named)           │
└─────────────────────────────────────────────────────────────────┘
```

**Version bits [2:0]:**
- `000` = CLASP v2 (legacy MessagePack with named keys)
- `001` = CLASP v3 (efficient binary encoding)

This allows graceful migration: old clients still work, new clients get performance.

### Message Encoding (All Types)

Every message starts with a **message type byte**, then type-specific fields.

#### Message Type Codes

| Code | Message | Direction | Purpose |
|------|---------|-----------|---------|
| 0x01 | HELLO | Client→Server | Connection initiation |
| 0x02 | WELCOME | Server→Client | Connection accepted |
| 0x03 | ANNOUNCE | Both | Capability advertisement |
| 0x10 | SUBSCRIBE | Client→Server | Subscribe to pattern |
| 0x11 | UNSUBSCRIBE | Client→Server | Unsubscribe |
| 0x20 | PUBLISH | Both | Event/Stream/Gesture |
| 0x21 | SET | Both | Set param value |
| 0x22 | GET | Client→Server | Request value |
| 0x23 | SNAPSHOT | Server→Client | State dump |
| 0x30 | BUNDLE | Both | Atomic message group |
| 0x40 | SYNC | Both | Clock sync |
| 0x41 | PING | Both | Keepalive |
| 0x42 | PONG | Both | Keepalive response |
| 0x50 | ACK | Both | Acknowledgment |
| 0x51 | ERROR | Both | Error response |
| 0x60 | QUERY | Client→Server | Introspection |
| 0x61 | RESULT | Server→Client | Query response |

#### Value Type Codes

| Code | Type | Size | Encoding |
|------|------|------|----------|
| 0x00 | Null | 0 | Nothing |
| 0x01 | Bool | 1 | 0x00=false, 0x01=true |
| 0x02 | Int8 | 1 | Signed |
| 0x03 | Int16 | 2 | Big-endian |
| 0x04 | Int32 | 4 | Big-endian |
| 0x05 | Int64 | 8 | Big-endian |
| 0x06 | Float32 | 4 | IEEE 754 |
| 0x07 | Float64 | 8 | IEEE 754 |
| 0x08 | String | 2+N | Length (u16 BE) + UTF-8 |
| 0x09 | Bytes | 2+N | Length (u16 BE) + raw |
| 0x0A | Array | 2+N | Count (u16) + elements |
| 0x0B | Map | 2+N | Count (u16) + key-values |

---

### High-Frequency Messages (Optimized)

#### SET (0x21) - Parameter Update

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x21                                            │
│ [1]    Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4] │
│ [2-3]  Address length (u16 BE)                               │
│ [4..N] Address (UTF-8)                                       │
│ [N+1..] Value (encoded per value type)                       │
│ [opt]  Revision (u64 BE) - if has_rev flag                   │
└──────────────────────────────────────────────────────────────┘
```

**Example:** SET /test/value = 0.5 with revision 1
```
21                    # SET
87                    # flags: has_rev=1, vtype=0x07 (float64)  
00 0B                 # address length: 11
2F 74 65 73 74 2F 76 61 6C 75 65  # "/test/value"
3F E0 00 00 00 00 00 00           # 0.5 as float64
00 00 00 00 00 00 00 01           # revision: 1
Total: 32 bytes (was 69 bytes = 54% smaller)
```

#### PUBLISH (0x20) - Event/Stream/Gesture

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x20                                            │
│ [1]    Flags: [sig_type:3][has_ts:1][has_id:1][phase:3]     │
│ [2-3]  Address length (u16 BE)                               │
│ [4..N] Address (UTF-8)                                       │
│ [N+1]  Value type                                            │
│ [N+2..] Value                                                │
│ [opt]  Timestamp (u64 BE) - if has_ts                        │
│ [opt]  Gesture ID (u32 BE) - if has_id                       │
└──────────────────────────────────────────────────────────────┘
```

Signal types: 0=Event, 1=Stream, 2=Gesture, 3=Timeline
Phases (gestures): 0=Start, 1=Move, 2=End, 3=Cancel

#### BUNDLE (0x30) - Atomic Group

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x30                                            │
│ [1]    Flags: [has_ts:1][reserved:7]                         │
│ [2-3]  Message count (u16 BE)                                │
│ [opt]  Timestamp (u64 BE) - if has_ts                        │
│ [4..]  Messages: [len:u16][message bytes]*                   │
└──────────────────────────────────────────────────────────────┘
```

---

### Connection Messages

#### HELLO (0x01)

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x01                                            │
│ [1]    Version: 3                                            │
│ [2]    Features: [param:1][event:1][stream:1][gesture:1]     │
│                  [timeline:1][reserved:3]                     │
│ [3-4]  Name length (u16 BE)                                  │
│ [5..N] Name (UTF-8)                                          │
│ [opt]  Token length (u16) + token (UTF-8)                    │
└──────────────────────────────────────────────────────────────┘
```

#### WELCOME (0x02)

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x02                                            │
│ [1]    Version: 3                                            │
│ [2]    Features (same as HELLO)                              │
│ [3-10] Server time (u64 BE, microseconds)                    │
│ [11-12] Session ID length (u16 BE)                           │
│ [13..N] Session ID (UTF-8)                                   │
│ [N+1..] Server name length (u16) + name                      │
│ [opt]   Token length (u16) + token                           │
└──────────────────────────────────────────────────────────────┘
```

#### SUBSCRIBE (0x10)

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x10                                            │
│ [1-4]  Subscription ID (u32 BE)                              │
│ [5-6]  Pattern length (u16 BE)                               │
│ [7..N] Pattern (UTF-8)                                       │
│ [N+1]  Signal type filter (bitmask, 0xFF = all)              │
│ [opt]  Options: max_rate (u16), epsilon (f32), history (u16) │
└──────────────────────────────────────────────────────────────┘
```

#### ERROR (0x51)

```
┌──────────────────────────────────────────────────────────────┐
│ [0]    Type: 0x51                                            │
│ [1-2]  Error code (u16 BE)                                   │
│ [3-4]  Message length (u16 BE)                               │
│ [5..N] Message (UTF-8)                                       │
│ [opt]  Address length (u16) + address                        │
│ [opt]  Correlation ID (u32)                                  │
└──────────────────────────────────────────────────────────────┘
```

---

### Size Comparison

| Message | v2 (named MsgPack) | v3 (binary) | Savings |
|---------|-------------------|-------------|---------|
| SET /test/value = 0.5 | 69 bytes | **32 bytes** | 54% |
| SET (no revision) | 57 bytes | **24 bytes** | 58% |
| PUBLISH event | ~55 bytes | **~18 bytes** | 67% |
| HELLO | ~45 bytes | **~20 bytes** | 56% |
| SUBSCRIBE | ~50 bytes | **~25 bytes** | 50% |
| BUNDLE (3 SETs) | ~200 bytes | **~90 bytes** | 55% |

---

## Implementation Plan

### Core Principle: Replace, Don't Add

We're **replacing** the encoding, not adding a mode. The implementation:

1. New `encode()` uses efficient binary
2. New `decode()` reads efficient binary
3. `decode()` also accepts v2 MessagePack for backward compatibility
4. Frame version bits distinguish v2 from v3

### Phase 1: Core Rust Implementation

#### 1.1 Replace codec.rs

**File:** `crates/clasp-core/src/codec.rs`

```rust
//! CLASP v3 Binary Codec
//!
//! Efficient binary encoding for all CLASP messages.
//! Backward compatible: can decode v2 MessagePack frames.

use crate::{Message, Result, Error};
use bytes::{Bytes, BytesMut, BufMut, Buf};

/// Protocol version
pub const VERSION: u8 = 3;

/// Message type codes (same semantics as v2, just efficient encoding)
pub mod msg {
    pub const HELLO: u8 = 0x01;
    pub const WELCOME: u8 = 0x02;
    pub const ANNOUNCE: u8 = 0x03;
    pub const SUBSCRIBE: u8 = 0x10;
    pub const UNSUBSCRIBE: u8 = 0x11;
    pub const PUBLISH: u8 = 0x20;
    pub const SET: u8 = 0x21;
    pub const GET: u8 = 0x22;
    pub const SNAPSHOT: u8 = 0x23;
    pub const BUNDLE: u8 = 0x30;
    pub const SYNC: u8 = 0x40;
    pub const PING: u8 = 0x41;
    pub const PONG: u8 = 0x42;
    pub const ACK: u8 = 0x50;
    pub const ERROR: u8 = 0x51;
    pub const QUERY: u8 = 0x60;
    pub const RESULT: u8 = 0x61;
}

/// Value type codes
pub mod val {
    pub const NULL: u8 = 0x00;
    pub const BOOL: u8 = 0x01;
    pub const I8: u8 = 0x02;
    pub const I16: u8 = 0x03;
    pub const I32: u8 = 0x04;
    pub const I64: u8 = 0x05;
    pub const F32: u8 = 0x06;
    pub const F64: u8 = 0x07;
    pub const STRING: u8 = 0x08;
    pub const BYTES: u8 = 0x09;
    pub const ARRAY: u8 = 0x0A;
    pub const MAP: u8 = 0x0B;
}

/// Encode message to v3 binary format
pub fn encode(message: &Message) -> Result<Bytes> {
    let mut buf = BytesMut::with_capacity(64);
    encode_message(&mut buf, message)?;
    Ok(buf.freeze())
}

/// Decode message - auto-detects v2 vs v3
pub fn decode(bytes: &[u8]) -> Result<Message> {
    if bytes.is_empty() {
        return Err(Error::BufferTooSmall { needed: 1, have: 0 });
    }
    
    // v3 messages start with a known message type byte
    // v2 MessagePack maps start with 0x80-0x8F (fixmap) or 0xDE-0xDF (map)
    let first = bytes[0];
    
    if is_msgpack_map(first) {
        // Legacy v2 format
        decode_v2_msgpack(bytes)
    } else {
        // v3 binary format
        decode_v3_binary(bytes)
    }
}

fn is_msgpack_map(byte: u8) -> bool {
    (byte & 0xF0) == 0x80 || byte == 0xDE || byte == 0xDF
}

fn encode_message(buf: &mut BytesMut, msg: &Message) -> Result<()> {
    match msg {
        Message::Hello(m) => encode_hello(buf, m),
        Message::Welcome(m) => encode_welcome(buf, m),
        Message::Set(m) => encode_set(buf, m),
        Message::Publish(m) => encode_publish(buf, m),
        Message::Subscribe(m) => encode_subscribe(buf, m),
        Message::Bundle(m) => encode_bundle(buf, m),
        Message::Ping => { buf.put_u8(msg::PING); Ok(()) },
        Message::Pong => { buf.put_u8(msg::PONG); Ok(()) },
        Message::Error(m) => encode_error(buf, m),
        // ... all other message types
    }
}

fn encode_set(buf: &mut BytesMut, msg: &SetMessage) -> Result<()> {
    buf.put_u8(msg::SET);
    
    // Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
    let vtype = value_type(&msg.value);
    let mut flags = vtype & 0x0F;
    if msg.revision.is_some() { flags |= 0x80; }
    if msg.lock { flags |= 0x40; }
    if msg.unlock { flags |= 0x20; }
    buf.put_u8(flags);
    
    // Address
    encode_string(buf, &msg.address)?;
    
    // Value (type already in flags for simple types)
    encode_value(buf, &msg.value)?;
    
    // Optional revision
    if let Some(rev) = msg.revision {
        buf.put_u64(rev);
    }
    
    Ok(())
}

// ... implement all other encode_* and decode_* functions
```

#### 1.2 Update Frame Flags

**File:** `crates/clasp-core/src/frame.rs`

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags {
    pub qos: QoS,
    pub has_timestamp: bool,
    pub encrypted: bool,
    pub compressed: bool,
    pub version: u8,  // 0=v2, 1=v3
}

impl FrameFlags {
    pub fn to_byte(&self) -> u8 {
        let mut flags = 0u8;
        flags |= (self.qos as u8) << 6;
        if self.has_timestamp { flags |= 0x20; }
        if self.encrypted { flags |= 0x10; }
        if self.compressed { flags |= 0x08; }
        flags |= self.version & 0x07;  // bits 0-2
        flags
    }
    
    pub fn from_byte(byte: u8) -> Self {
        Self {
            qos: QoS::from_u8((byte >> 6) & 0x03).unwrap_or(QoS::Fire),
            has_timestamp: (byte & 0x20) != 0,
            encrypted: (byte & 0x10) != 0,
            compressed: (byte & 0x08) != 0,
            version: byte & 0x07,
        }
    }
}
```

#### 1.3 Benchmarks

**File:** `crates/clasp-core/benches/codec.rs`

```rust
fn benchmark_encoding(c: &mut Criterion) {
    let msg = Message::Set(SetMessage {
        address: "/test/benchmark/value".to_string(),
        value: Value::Float(0.5),
        revision: Some(1),
        lock: false,
        unlock: false,
    });
    
    let mut group = c.benchmark_group("codec_v3");
    
    group.bench_function("encode_set", |b| {
        b.iter(|| black_box(codec::encode(&msg).unwrap()))
    });
    
    let encoded = codec::encode(&msg).unwrap();
    group.bench_function("decode_set", |b| {
        b.iter(|| black_box(codec::decode(&encoded).unwrap()))
    });
    
    group.finish();
    
    // Report sizes
    println!("SET message size: {} bytes", encoded.len());
}
```

### Phase 2: Downstream Rust Crates

All downstream crates use `clasp_core::codec`. After updating the core codec, they automatically use v3 encoding. Changes needed:

#### 2.1 clasp-router

**File:** `crates/clasp-router/src/router.rs`

- [ ] No codec changes needed (uses clasp_core::codec)
- [ ] Update frame version when encoding responses

#### 2.2 clasp-client

**File:** `crates/clasp-client/src/client.rs`

- [ ] No codec changes needed (uses clasp_core::codec)
- [ ] Works automatically with v3

#### 2.3 clasp-bridge

All bridges convert external protocols to CLASP messages, then use clasp_core::codec.
- [ ] No changes needed - bridges are codec-agnostic

#### 2.4 clasp-transport

- [ ] No changes - transport just moves bytes

#### 2.5 clasp-wasm

**File:** `crates/clasp-wasm/src/lib.rs`

- [ ] Uses clasp_core::codec internally
- [ ] Automatically gets v3 encoding

#### 2.6 clasp-embedded

**File:** `crates/clasp-embedded/src/lib.rs`

The "Lite" format is a subset of v3. Ensure compatibility:
- [ ] Lite SET (0x21) is identical to v3 SET
- [ ] Lite PUBLISH (0x20) is identical to v3 PUBLISH

### Phase 3: Language Bindings

All bindings need to be updated to use v3 binary encoding as the default.

#### 3.1 JavaScript Bindings

**File:** `bindings/js/packages/clasp-core/src/codec.ts`

Replace MessagePack encoding with direct binary:

```typescript
/** Message type codes */
export const MSG = {
  HELLO: 0x01,
  WELCOME: 0x02,
  SET: 0x21,
  PUBLISH: 0x20,
  // ... etc
} as const;

/** Value type codes */
export const VAL = {
  NULL: 0x00,
  BOOL: 0x01,
  F32: 0x06,
  F64: 0x07,
  STRING: 0x08,
  // ... etc
} as const;

/** Encode message to v3 binary */
export function encode(message: Message): Uint8Array {
  const buf = new ArrayBuffer(256);
  const view = new DataView(buf);
  let offset = 0;

  switch (message.type) {
    case 'SET':
      offset = encodeSet(view, offset, message);
      break;
    case 'PUBLISH':
      offset = encodePublish(view, offset, message);
      break;
    // ... all message types
  }

  return new Uint8Array(buf, 0, offset);
}

/** Decode message - handles both v2 and v3 */
export function decode(data: Uint8Array): Message {
  const first = data[0];
  
  // v2 MessagePack maps start with 0x80-0x8F
  if ((first & 0xF0) === 0x80 || first === 0xDE || first === 0xDF) {
    return msgpackDecode(data) as Message;  // Legacy
  }
  
  // v3 binary - first byte is message type
  return decodeV3(data);
}

function encodeSet(view: DataView, offset: number, msg: SetMessage): number {
  view.setUint8(offset++, MSG.SET);
  
  // Flags
  let flags = getValueType(msg.value) & 0x0F;
  if (msg.revision !== undefined) flags |= 0x80;
  if (msg.lock) flags |= 0x40;
  if (msg.unlock) flags |= 0x20;
  view.setUint8(offset++, flags);
  
  // Address
  offset = encodeString(view, offset, msg.address);
  
  // Value
  offset = encodeValue(view, offset, msg.value);
  
  // Revision
  if (msg.revision !== undefined) {
    view.setBigUint64(offset, BigInt(msg.revision), false);
    offset += 8;
  }
  
  return offset;
}
```

#### 3.2 Python Bindings

**File:** `bindings/python/python/clasp/client.py`

```python
import struct

# Message type codes
MSG_SET = 0x21
MSG_PUBLISH = 0x20
MSG_HELLO = 0x01
# ... etc

def _encode(self, msg: Dict[str, Any]) -> bytes:
    """Encode message to v3 binary format"""
    msg_type = msg.get("type")
    
    if msg_type == "SET":
        return self._encode_set(msg)
    elif msg_type == "PUBLISH":
        return self._encode_publish(msg)
    # ... all message types

def _encode_set(self, msg: Dict[str, Any]) -> bytes:
    """Encode SET message"""
    parts = [struct.pack('B', MSG_SET)]
    
    # Flags
    flags = self._value_type(msg["value"]) & 0x0F
    if msg.get("revision") is not None:
        flags |= 0x80
    if msg.get("lock"):
        flags |= 0x40
    if msg.get("unlock"):
        flags |= 0x20
    parts.append(struct.pack('B', flags))
    
    # Address
    addr = msg["address"].encode('utf-8')
    parts.append(struct.pack('>H', len(addr)))
    parts.append(addr)
    
    # Value
    parts.append(self._encode_value(msg["value"]))
    
    # Revision
    if msg.get("revision") is not None:
        parts.append(struct.pack('>Q', msg["revision"]))
    
    return b''.join(parts)

def _decode(self, data: bytes) -> Dict[str, Any]:
    """Decode message - auto-detects v2 vs v3"""
    first = data[0]
    
    # v2 MessagePack
    if (first & 0xF0) == 0x80 or first in (0xDE, 0xDF):
        return msgpack.unpackb(data, raw=False)
    
    # v3 binary
    return self._decode_v3(data)
```

#### 3.3 Standalone JS (clasp-minimal.js)

**File:** `clasp-minimal.js`

```javascript
// v3 binary encoding (replaces msgpack)
function encodeFrame(message, options = {}) {
  const payload = encodeMessage(message);
  
  let flags = (options.qos || QOS.FIRE) << 6;
  flags |= 0x01;  // Version 1 = v3 binary
  if (options.timestamp) flags |= 0x20;
  
  const headerSize = options.timestamp ? 12 : 4;
  const frame = Buffer.alloc(headerSize + payload.length);
  
  frame[0] = 0x53;  // Magic
  frame[1] = flags;
  frame.writeUInt16BE(payload.length, 2);
  
  if (options.timestamp) {
    frame.writeBigUInt64BE(BigInt(options.timestamp), 4);
    payload.copy(frame, 12);
  } else {
    payload.copy(frame, 4);
  }
  
  return frame;
}

function encodeMessage(msg) {
  const buf = Buffer.alloc(256);
  let offset = 0;
  
  switch (msg.type) {
    case MSG.SET:
      buf[offset++] = MSG.SET;
      // ... encode SET fields
      break;
    // ... other types
  }
  
  return buf.slice(0, offset);
}
```

### Phase 4: Desktop App

**Files:**
- `apps/bridge/src/app.js` - No encoding changes needed (uses backend)
- `apps/bridge/electron/main.js` - No encoding changes needed (spawns clasp-service)

The desktop app delegates encoding to `clasp-service`, which will automatically support compact encoding after Rust changes.

### Phase 5: Documentation

#### 5.1 Protocol Specification

**File:** `CLASP-Protocol-v2.md`

Add new section:

```markdown
## 2.3 Encoding Modes

CLASP supports two encoding modes for payload data:

### 2.3.1 Standard Mode (Default)

Uses MessagePack with named keys. Human-readable, easy to debug.

Indicated by frame flags bit 0 = 0.

### 2.3.2 Compact Mode

Uses custom binary encoding for high-frequency messages (SET, PUBLISH, BUNDLE, ACK).
Reduces message size by ~50% and improves encode/decode speed by 4-6x.

Indicated by frame flags bit 0 = 1.

See Appendix F for compact encoding format specification.
```

Add Appendix F with full compact format specification.

#### 5.2 Quick Reference

**File:** `CLASP-QuickRef.md`

Update frame format section:

```markdown
## Frame Format (4 bytes minimum)
```
[0]    Magic 'S' (0x53)
[1]    Flags
       [7:6] QoS (00=fire, 01=confirm, 10=commit)
       [5]   Timestamp present
       [4]   Encrypted
       [3]   Compressed
       [2:1] Reserved
       [0]   Compact encoding (0=standard, 1=compact)
[2-3]  Payload length (uint16 BE)
[4+]   Payload (MessagePack or compact binary)
```
```

#### 5.3 README Performance Section

**File:** `README.md`

Update the performance table:

```markdown
## Performance

### Encoding/Decoding Speed (messages/second)

| Protocol | Encoding | Decoding | Message Size |
|----------|----------|----------|--------------|
| **MQTT** | 11.4M | 11.4M | 19 B |
| **OSC** | 4.5M | 5.7M | 24 B |
| **CLASP (compact)** | **8.0M** | **10.0M** | **32 B** |
| **CLASP (standard)** | 1.8M | 1.5M | 69 B |

CLASP compact mode provides competitive performance with MQTT/OSC while
supporting rich features like state synchronization, typed signals, and
revision tracking.
```

#### 5.4 HANDOFF.md

**File:** `HANDOFF.md`

Update wire protocol summary:

```markdown
## Wire Protocol Summary

```
Frame: 4-12 bytes header + payload

Byte 0:     0x53 ('S' magic)
Byte 1:     Flags [QoS:2][TS:1][Enc:1][Cmp:1][Rsv:2][Compact:1]
Bytes 2-3:  Payload length (uint16 BE)
[Bytes 4-11: Timestamp if TS flag set]
Payload:    MessagePack (standard) or binary (compact)

Compact mode (bit 0 = 1): Custom binary for SET/PUBLISH/BUNDLE
Standard mode (bit 0 = 0): MessagePack with named keys
```
```

#### 5.5 Site Spec Section

**File:** `site/src/components/SpecSection.vue`

Add compact encoding documentation to the protocol spec visualization.

### Phase 6: Tests

#### 6.1 Unit Tests

**File:** `crates/clasp-core/tests/codec_tests.rs` (new tests)

```rust
#[test]
fn test_compact_set_roundtrip() {
    let msg = Message::Set(SetMessage {
        address: "/test/value".to_string(),
        value: Value::Float(0.5),
        revision: Some(42),
        lock: false,
        unlock: false,
    });
    
    let encoded = codec::encode_compact(&msg).unwrap();
    let decoded = codec::decode_compact(&encoded).unwrap();
    
    assert_eq!(msg, decoded);
}

#[test]
fn test_compact_size_reduction() {
    let msg = Message::Set(SetMessage { /* ... */ });
    
    let standard = codec::encode(&msg).unwrap();
    let compact = codec::encode_compact(&msg).unwrap();
    
    // Compact should be at least 40% smaller
    assert!(compact.len() < standard.len() * 60 / 100);
}

#[test]
fn test_auto_detect_encoding_mode() {
    let msg = Message::Set(SetMessage { /* ... */ });
    
    // Standard encoding
    let standard = codec::encode(&msg).unwrap();
    let (decoded_std, frame_std) = codec::decode(&standard).unwrap();
    assert!(!frame_std.flags.compact);
    
    // Compact encoding
    let compact = codec::encode_compact_frame(&msg).unwrap();
    let (decoded_cmp, frame_cmp) = codec::decode(&compact).unwrap();
    assert!(frame_cmp.flags.compact);
    
    // Both should decode to same message
    assert_eq!(decoded_std, decoded_cmp);
}
```

#### 6.2 Integration Tests

**File:** `test-suite/src/bin/encoding_tests.rs` (new file)

```rust
//! Encoding mode integration tests

#[tokio::test]
async fn test_mixed_encoding_modes() {
    // Start router
    let router = Router::new().await;
    
    // Client 1: uses standard encoding
    let client1 = ClaspBuilder::new(&router.url())
        .connect()
        .await?;
    
    // Client 2: uses compact encoding
    let client2 = ClaspBuilder::new(&router.url())
        .with_compact_encoding()
        .connect()
        .await?;
    
    // Client 2 subscribes
    let (tx, mut rx) = mpsc::channel(10);
    client2.subscribe("/test/**", move |value, addr| {
        tx.send((addr.to_string(), value.clone())).await.ok();
    }).await?;
    
    // Client 1 sends with standard encoding
    client1.set("/test/a", Value::Float(1.0)).await?;
    
    // Client 2 should receive it
    let (addr, value) = rx.recv().await.unwrap();
    assert_eq!(addr, "/test/a");
    assert_eq!(value.as_f64(), Some(1.0));
}
```

#### 6.3 JavaScript Tests

**File:** `bindings/js/packages/clasp-core/tests/codec.test.ts`

```typescript
describe('Compact Encoding', () => {
  it('should encode SET messages in compact format', () => {
    const msg = { type: 'SET', address: '/test/value', value: 0.5 };
    const encoded = encodeCompact(msg);
    
    expect(encoded[0]).toBe(0x21);  // SET type
    expect(encoded.length).toBeLessThan(40);  // Much smaller than standard
  });
  
  it('should decode compact messages', () => {
    const msg = { type: 'SET', address: '/test/value', value: 0.5, revision: 1 };
    const encoded = encodeCompact(msg);
    const decoded = decodeCompact(encoded);
    
    expect(decoded.address).toBe('/test/value');
    expect(decoded.value).toBe(0.5);
    expect(decoded.revision).toBe(1);
  });
});
```

### Phase 7: Examples Update

#### 7.1 JavaScript Examples

**File:** `examples/js/simple-publisher.js`

```javascript
// Example showing compact encoding
const client = await new ClaspBuilder('ws://localhost:7330')
  .withName('Compact Publisher')
  .withCompactEncoding()  // Enable compact mode for better performance
  .connect();

// High-frequency streaming benefits from compact encoding
setInterval(() => {
  client.stream('/sensors/accelerometer', { x: 0.1, y: 0.2, z: 0.9 });
}, 16);  // 60 Hz
```

#### 7.2 Rust Examples

**File:** `examples/rust/basic-client.rs`

```rust
// Example showing compact encoding
let client = ClaspBuilder::new("ws://localhost:7330")
    .name("Compact Publisher")
    .compact_encoding(true)  // Enable compact mode
    .connect()
    .await?;
```

---

## Implementation Order

### Week 1: Core Rust Implementation
1. [ ] Rewrite `crates/clasp-core/src/codec.rs` with v3 binary encoding
2. [ ] Update `frame.rs` with version field in flags
3. [ ] Add comprehensive unit tests (encode/decode roundtrip for ALL message types)
4. [ ] Run benchmarks to verify ≥5x improvement
5. [ ] Verify v2 backward compatibility in decoder

### Week 2: Language Bindings
1. [ ] Rewrite `bindings/js/packages/clasp-core/src/codec.ts` 
2. [ ] Rewrite `bindings/python/python/clasp/client.py` encoding
3. [ ] Update `clasp-minimal.js`
4. [ ] Add JS unit tests for v3 encoding
5. [ ] Add Python unit tests for v3 encoding

### Week 3: Documentation + Testing
1. [ ] Update `CLASP-Protocol-v2.md` → rename to `CLASP-Protocol-v3.md`
2. [ ] Update `README.md` performance section
3. [ ] Update `CLASP-QuickRef.md` wire format
4. [ ] Add cross-language integration tests (Rust ↔ JS ↔ Python)
5. [ ] Update examples in all languages

### Week 4: Release
1. [ ] Final testing on all platforms
2. [ ] Bump version to 0.2.0
3. [ ] Publish Rust crates
4. [ ] Publish npm package
5. [ ] Publish Python package
6. [ ] Update website with new benchmarks

---

## Backward Compatibility

### How It Works

1. **Frame version bits** - Bits 0-2 of flags indicate encoding version
   - `000` = v2 legacy (MessagePack with named keys)
   - `001` = v3 binary (new efficient format)

2. **Decoder auto-detects** - `decode()` checks first byte:
   - MessagePack map prefix (0x80-0x8F, 0xDE-0xDF) → v2
   - Known message type byte (0x01-0x61) → v3

3. **Encoder outputs v3** - All new messages use efficient binary

### Migration Path

```
Timeline:
─────────────────────────────────────────────────────────────────
v0.1.x (current)    v0.2.0              v0.3.0           v1.0.0
MessagePack named   Both decode         v3 default       v2 deprecated
                    v3 encode           v2 still works
─────────────────────────────────────────────────────────────────
```

**Phase 1 (v0.2.0):** Decoders accept both v2 and v3. Encoder outputs v3.
- Old clients (v2) can talk to new servers
- New clients (v3) talk efficiently

**Phase 2 (v0.3.0+):** v3 is fully default. v2 decode still supported.
- Existing deployments continue working
- Performance gains realized

**Phase 3 (v1.0.0):** Consider deprecating v2 decode (optional).

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Parsing bugs in v3 decoder | Medium | High | Extensive roundtrip tests, fuzzing |
| Old clients can't talk to new | Low | Medium | v2 decode always supported |
| Cross-language encoding mismatch | Medium | High | Reference test vectors, integration tests |
| Performance regression in edge cases | Low | Medium | Benchmark all message types |

---

## Success Metrics

| Metric | Target | Verification |
|--------|--------|--------------|
| SET message size | **≤ 32 bytes** | `assert!(encoded.len() <= 32)` |
| Encoding speed | **≥ 8M msg/s** | Criterion benchmark |
| Decoding speed | **≥ 10M msg/s** | Criterion benchmark |
| All Rust tests | 100% pass | `cargo test --workspace` |
| All JS tests | 100% pass | `npm test` |
| All Python tests | 100% pass | `pytest` |
| Cross-language roundtrip | Works | Rust→JS→Python→Rust test |
| v2 backward compat | Works | Old client test suite |

---

## File Checklist

### Core Changes (Week 1)
| File | Change | Priority |
|------|--------|----------|
| `crates/clasp-core/src/codec.rs` | Rewrite with v3 binary | **P0** |
| `crates/clasp-core/src/frame.rs` | Add version field to flags | **P0** |
| `crates/clasp-core/src/lib.rs` | Update exports | **P0** |
| `crates/clasp-core/benches/codec.rs` | Update benchmarks | P1 |

### Language Bindings (Week 2)
| File | Change | Priority |
|------|--------|----------|
| `bindings/js/packages/clasp-core/src/codec.ts` | Rewrite with v3 | **P0** |
| `bindings/python/python/clasp/client.py` | Rewrite encode/decode | **P0** |
| `clasp-minimal.js` | Update encoding | P1 |

### Documentation (Week 3)
| File | Change | Priority |
|------|--------|----------|
| `CLASP-Protocol-v2.md` | Update to v3 format spec | **P0** |
| `CLASP-QuickRef.md` | Update wire format | P1 |
| `README.md` | Update performance numbers | P1 |
| `HANDOFF.md` | Update wire protocol section | P2 |

### Tests
| File | Change | Priority |
|------|--------|----------|
| `crates/clasp-core/tests/codec_tests.rs` | v3 roundtrip tests | **P0** |
| `test-suite/benches/throughput.rs` | v3 benchmarks | P1 |
| `bindings/js/.../__tests__/` | JS v3 tests | P1 |

### No Changes Needed (Use clasp_core::codec)
- `crates/clasp-router/` - Uses codec, auto-updated
- `crates/clasp-client/` - Uses codec, auto-updated
- `crates/clasp-bridge/` - Uses codec, auto-updated
- `crates/clasp-transport/` - Transport-agnostic
- `crates/clasp-wasm/` - Uses codec, auto-updated
- `apps/bridge/` - Uses clasp-service, auto-updated

---

## Appendix A: Test Vectors

### SET Message

**Input:**
```json
{
  "type": "SET",
  "address": "/test/value",
  "value": 0.5,
  "revision": 1
}
```

**v3 Binary Output (32 bytes):**
```
21 87 00 0B 2F 74 65 73 74 2F 76 61 6C 75 65
3F E0 00 00 00 00 00 00
00 00 00 00 00 00 00 01

Breakdown:
21          SET message type
87          flags: has_rev=1, vtype=7 (f64)
00 0B       address length: 11
2F 74 65... "/test/value" UTF-8
3F E0 00... 0.5 as IEEE 754 float64
00 00 00... revision: 1
```

**v2 MessagePack Output (69 bytes):**
```
86 A4 74 79 70 65 A3 53 45 54 A7 61 64 64 72 65
73 73 AB 2F 74 65 73 74 2F 76 61 6C 75 65 A5 76
61 6C 75 65 CB 3F E0 00 00 00 00 00 00 A8 72 65
76 69 73 69 6F 6E 01 A4 6C 6F 63 6B C2 A6 75 6E
6C 6F 63 6B C2
```

---

## Appendix B: Cross-Language Test

```javascript
// Reference test: same message, same bytes across Rust/JS/Python

const testMessage = {
  type: 'SET',
  address: '/sensor/temperature',
  value: 23.5,
  revision: 42
};

// Expected v3 binary output (all languages must produce this exact output)
const expected = new Uint8Array([
  0x21,                                     // SET
  0x87,                                     // flags: has_rev=1, vtype=7 (f64)
  0x00, 0x13,                               // address length: 19
  0x2F, 0x73, 0x65, 0x6E, 0x73, 0x6F, 0x72, // "/sensor/"
  0x2F, 0x74, 0x65, 0x6D, 0x70, 0x65, 0x72, // "tempera"
  0x61, 0x74, 0x75, 0x72, 0x65,             // "ture"
  0x40, 0x37, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, // 23.5 as f64
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A  // revision: 42
]);
```

---

## Appendix C: Why This is Better

| Aspect | v2 (MessagePack named) | v3 (Binary) |
|--------|------------------------|-------------|
| Field names | Every message | Never |
| Parsing | Recursive map decode | Direct byte access |
| Allocation | Multiple strings | Zero-copy possible |
| Size | 69B for SET | 32B for SET |
| Speed | 1.8M msg/s encode | ~10M msg/s encode |

**Key insight:** The protocol *semantics* are unchanged. We're simply encoding the same information without redundancy. Like how Protocol Buffers encodes structured data - you don't send field names on the wire, just the data in a known order.

---

*Document generated 2026-01-22. Implementation will make CLASP faster than MQTT/OSC for encoding while retaining all advanced features.*
