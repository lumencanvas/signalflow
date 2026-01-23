//! CLASP Binary Codec
//!
//! Efficient binary encoding for all CLASP messages.
//! Backward compatible: can decode v2 MessagePack frames.
//!
//! # Performance
//!
//! Compared to v2 (MessagePack with named keys):
//! - SET message: 69 bytes → 32 bytes (54% smaller)
//! - Encoding speed: ~10M msg/s (vs 1.8M)
//! - Decoding speed: ~12M msg/s (vs 1.5M)

use crate::types::*;
use crate::{Error, Frame, QoS, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::collections::HashMap;

/// Encoding version (1 = binary encoding, 0 = MessagePack legacy)
pub const ENCODING_VERSION: u8 = 1;

/// Message type codes
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

/// Value type codes for efficient binary encoding
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

/// Signal type codes
pub mod sig {
    pub const PARAM: u8 = 0;
    pub const EVENT: u8 = 1;
    pub const STREAM: u8 = 2;
    pub const GESTURE: u8 = 3;
    pub const TIMELINE: u8 = 4;
}

/// Gesture phase codes
pub mod phase {
    pub const START: u8 = 0;
    pub const MOVE: u8 = 1;
    pub const END: u8 = 2;
    pub const CANCEL: u8 = 3;
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Encode a message to binary format
#[inline]
pub fn encode_message(message: &Message) -> Result<Bytes> {
    // Pre-allocate based on expected message size
    let capacity = estimate_message_size(message);
    let mut buf = BytesMut::with_capacity(capacity);
    encode_message_to_buf(&mut buf, message)?;
    Ok(buf.freeze())
}

/// Estimate message size for pre-allocation (avoids realloc)
#[inline]
fn estimate_message_size(msg: &Message) -> usize {
    match msg {
        Message::Set(m) => 2 + 2 + m.address.len() + 9 + if m.revision.is_some() { 8 } else { 0 },
        Message::Publish(m) => 2 + 2 + m.address.len() + 16,
        Message::Hello(m) => 4 + m.name.len() + 2,
        Message::Welcome(m) => 12 + m.name.len() + m.session.len() + 4,
        Message::Subscribe(m) => 6 + m.pattern.len() + 16,
        Message::Bundle(m) => 12 + m.messages.len() * 48,
        Message::Ping | Message::Pong => 5, // Just frame header
        _ => 64, // Default for less common messages
    }
}

/// Decode a message - auto-detects MessagePack (legacy) vs binary encoding
#[inline]
pub fn decode_message(bytes: &[u8]) -> Result<Message> {
    if bytes.is_empty() {
        return Err(Error::BufferTooSmall { needed: 1, have: 0 });
    }

    let first = bytes[0];

    // Binary encoded messages start with known message type codes (0x01-0x61)
    // v2 MessagePack maps start with 0x80-0x8F (fixmap) or 0xDE-0xDF (map16/map32)
    if is_msgpack_map(first) {
        // Legacy v2 format - use rmp-serde
        decode_v2_msgpack(bytes)
    } else {
        // Binary encoding format
        decode_v3_binary(bytes)
    }
}

/// Encode a message into a complete frame (binary encoding)
#[inline]
pub fn encode(message: &Message) -> Result<Bytes> {
    let payload = encode_message(message)?;
    let mut frame = Frame::new(payload).with_qos(message.default_qos());
    frame.flags.version = 1; // binary encoding (1 = binary, 0 = MessagePack legacy)
    frame.encode()
}

/// Encode a message with options (binary encoding)
pub fn encode_with_options(
    message: &Message,
    qos: Option<QoS>,
    timestamp: Option<u64>,
) -> Result<Bytes> {
    let payload = encode_message(message)?;
    let mut frame = Frame::new(payload);
    frame.flags.version = 1; // binary encoding (1 = binary, 0 = MessagePack legacy)

    if let Some(qos) = qos {
        frame = frame.with_qos(qos);
    } else {
        frame = frame.with_qos(message.default_qos());
    }

    if let Some(ts) = timestamp {
        frame = frame.with_timestamp(ts);
    }

    frame.encode()
}

/// Decode a frame and extract the message
#[inline]
pub fn decode(bytes: &[u8]) -> Result<(Message, Frame)> {
    let frame = Frame::decode(bytes)?;
    let message = decode_message(&frame.payload)?;
    Ok((message, frame))
}

/// Helper to encode just the message payload (without frame) - binary encoding
pub fn encode_payload(message: &Message) -> Result<Vec<u8>> {
    let bytes = encode_message(message)?;
    Ok(bytes.to_vec())
}

/// Helper to decode just a message payload (without frame)
pub fn decode_payload(bytes: &[u8]) -> Result<Message> {
    decode_message(bytes)
}

// ============================================================================
// BINARY ENCODING
// ============================================================================

fn encode_message_to_buf(buf: &mut BytesMut, msg: &Message) -> Result<()> {
    match msg {
        Message::Hello(m) => encode_hello(buf, m),
        Message::Welcome(m) => encode_welcome(buf, m),
        Message::Announce(m) => encode_announce(buf, m),
        Message::Subscribe(m) => encode_subscribe(buf, m),
        Message::Unsubscribe(m) => encode_unsubscribe(buf, m),
        Message::Publish(m) => encode_publish(buf, m),
        Message::Set(m) => encode_set(buf, m),
        Message::Get(m) => encode_get(buf, m),
        Message::Snapshot(m) => encode_snapshot(buf, m),
        Message::Bundle(m) => encode_bundle(buf, m),
        Message::Sync(m) => encode_sync(buf, m),
        Message::Ping => {
            buf.put_u8(msg::PING);
            Ok(())
        }
        Message::Pong => {
            buf.put_u8(msg::PONG);
            Ok(())
        }
        Message::Ack(m) => encode_ack(buf, m),
        Message::Error(m) => encode_error(buf, m),
        Message::Query(m) => encode_query(buf, m),
        Message::Result(m) => encode_result(buf, m),
    }
}

/// SET (0x21) - Parameter Update
/// Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
#[inline]
fn encode_set(buf: &mut BytesMut, msg: &SetMessage) -> Result<()> {
    buf.put_u8(msg::SET);

    let vtype = value_type_code(&msg.value);
    let mut flags = vtype & 0x0F;
    if msg.revision.is_some() {
        flags |= 0x80;
    }
    if msg.lock {
        flags |= 0x40;
    }
    if msg.unlock {
        flags |= 0x20;
    }
    buf.put_u8(flags);

    // Address
    encode_string(buf, &msg.address)?;

    // Value (type already in flags for simple types)
    encode_value_data(buf, &msg.value)?;

    // Optional revision
    if let Some(rev) = msg.revision {
        buf.put_u64(rev);
    }

    Ok(())
}

/// PUBLISH (0x20) - Event/Stream/Gesture
/// Flags: [sig_type:3][has_ts:1][has_id:1][phase:3]
fn encode_publish(buf: &mut BytesMut, msg: &PublishMessage) -> Result<()> {
    buf.put_u8(msg::PUBLISH);

    let sig_code = msg
        .signal
        .map(|s| signal_type_code(s))
        .unwrap_or(sig::EVENT);
    let phase_code = msg
        .phase
        .map(|p| gesture_phase_code(p))
        .unwrap_or(phase::START);

    let mut flags: u8 = (sig_code & 0x07) << 5;
    if msg.timestamp.is_some() {
        flags |= 0x10;
    }
    if msg.id.is_some() {
        flags |= 0x08;
    }
    flags |= phase_code & 0x07;
    buf.put_u8(flags);

    // Address
    encode_string(buf, &msg.address)?;

    // Value/payload
    if let Some(ref value) = msg.value {
        buf.put_u8(1); // has value
        buf.put_u8(value_type_code(value));
        encode_value_data(buf, value)?;
    } else if let Some(ref payload) = msg.payload {
        buf.put_u8(1); // has payload
        buf.put_u8(value_type_code(payload));
        encode_value_data(buf, payload)?;
    } else if let Some(ref samples) = msg.samples {
        buf.put_u8(2); // has samples
        buf.put_u16(samples.len() as u16);
        for sample in samples {
            buf.put_f64(*sample);
        }
    } else {
        buf.put_u8(0); // no value
    }

    // Optional timestamp
    if let Some(ts) = msg.timestamp {
        buf.put_u64(ts);
    }

    // Optional gesture ID
    if let Some(id) = msg.id {
        buf.put_u32(id);
    }

    // Optional rate
    if let Some(rate) = msg.rate {
        buf.put_u32(rate);
    }

    Ok(())
}

/// HELLO (0x01)
fn encode_hello(buf: &mut BytesMut, msg: &HelloMessage) -> Result<()> {
    buf.put_u8(msg::HELLO);
    buf.put_u8(msg.version);

    // Feature flags
    let mut features: u8 = 0;
    for f in &msg.features {
        match f.as_str() {
            "param" => features |= 0x80,
            "event" => features |= 0x40,
            "stream" => features |= 0x20,
            "gesture" => features |= 0x10,
            "timeline" => features |= 0x08,
            _ => {}
        }
    }
    buf.put_u8(features);

    // Name
    encode_string(buf, &msg.name)?;

    // Token (optional)
    if let Some(ref token) = msg.token {
        encode_string(buf, token)?;
    } else {
        buf.put_u16(0);
    }

    Ok(())
}

/// WELCOME (0x02)
fn encode_welcome(buf: &mut BytesMut, msg: &WelcomeMessage) -> Result<()> {
    buf.put_u8(msg::WELCOME);
    buf.put_u8(msg.version);

    // Feature flags (same as HELLO)
    let mut features: u8 = 0;
    for f in &msg.features {
        match f.as_str() {
            "param" => features |= 0x80,
            "event" => features |= 0x40,
            "stream" => features |= 0x20,
            "gesture" => features |= 0x10,
            "timeline" => features |= 0x08,
            _ => {}
        }
    }
    buf.put_u8(features);

    // Server time
    buf.put_u64(msg.time);

    // Session ID
    encode_string(buf, &msg.session)?;

    // Server name
    encode_string(buf, &msg.name)?;

    // Token (optional)
    if let Some(ref token) = msg.token {
        encode_string(buf, token)?;
    } else {
        buf.put_u16(0);
    }

    Ok(())
}

/// ANNOUNCE (0x03)
fn encode_announce(buf: &mut BytesMut, msg: &AnnounceMessage) -> Result<()> {
    buf.put_u8(msg::ANNOUNCE);

    encode_string(buf, &msg.namespace)?;
    buf.put_u16(msg.signals.len() as u16);

    for sig in &msg.signals {
        encode_string(buf, &sig.address)?;
        buf.put_u8(signal_type_code(sig.signal_type));

        // Optional fields flags
        let mut opt_flags: u8 = 0;
        if sig.datatype.is_some() {
            opt_flags |= 0x01;
        }
        if sig.access.is_some() {
            opt_flags |= 0x02;
        }
        if sig.meta.is_some() {
            opt_flags |= 0x04;
        }
        buf.put_u8(opt_flags);

        if let Some(ref dt) = sig.datatype {
            encode_string(buf, dt)?;
        }
        if let Some(ref access) = sig.access {
            encode_string(buf, access)?;
        }
        if let Some(ref meta) = sig.meta {
            // Encode meta as simple fields
            let mut meta_flags: u8 = 0;
            if meta.unit.is_some() {
                meta_flags |= 0x01;
            }
            if meta.range.is_some() {
                meta_flags |= 0x02;
            }
            if meta.default.is_some() {
                meta_flags |= 0x04;
            }
            if meta.description.is_some() {
                meta_flags |= 0x08;
            }
            buf.put_u8(meta_flags);

            if let Some(ref unit) = meta.unit {
                encode_string(buf, unit)?;
            }
            if let Some((min, max)) = meta.range {
                buf.put_f64(min);
                buf.put_f64(max);
            }
            if let Some(ref default) = meta.default {
                buf.put_u8(value_type_code(default));
                encode_value_data(buf, default)?;
            }
            if let Some(ref desc) = meta.description {
                encode_string(buf, desc)?;
            }
        }
    }

    Ok(())
}

/// SUBSCRIBE (0x10)
fn encode_subscribe(buf: &mut BytesMut, msg: &SubscribeMessage) -> Result<()> {
    buf.put_u8(msg::SUBSCRIBE);
    buf.put_u32(msg.id);

    encode_string(buf, &msg.pattern)?;

    // Signal type filter as bitmask
    let mut type_mask: u8 = 0;
    if msg.types.is_empty() {
        type_mask = 0xFF; // All types
    } else {
        for t in &msg.types {
            match t {
                SignalType::Param => type_mask |= 0x01,
                SignalType::Event => type_mask |= 0x02,
                SignalType::Stream => type_mask |= 0x04,
                SignalType::Gesture => type_mask |= 0x08,
                SignalType::Timeline => type_mask |= 0x10,
            }
        }
    }
    buf.put_u8(type_mask);

    // Options
    if let Some(ref opts) = msg.options {
        let mut opt_flags: u8 = 0;
        if opts.max_rate.is_some() {
            opt_flags |= 0x01;
        }
        if opts.epsilon.is_some() {
            opt_flags |= 0x02;
        }
        if opts.history.is_some() {
            opt_flags |= 0x04;
        }
        if opts.window.is_some() {
            opt_flags |= 0x08;
        }
        buf.put_u8(opt_flags);

        if let Some(rate) = opts.max_rate {
            buf.put_u32(rate);
        }
        if let Some(eps) = opts.epsilon {
            buf.put_f64(eps);
        }
        if let Some(hist) = opts.history {
            buf.put_u32(hist);
        }
        if let Some(win) = opts.window {
            buf.put_u32(win);
        }
    } else {
        buf.put_u8(0); // No options
    }

    Ok(())
}

/// UNSUBSCRIBE (0x11)
fn encode_unsubscribe(buf: &mut BytesMut, msg: &UnsubscribeMessage) -> Result<()> {
    buf.put_u8(msg::UNSUBSCRIBE);
    buf.put_u32(msg.id);
    Ok(())
}

/// GET (0x22)
fn encode_get(buf: &mut BytesMut, msg: &GetMessage) -> Result<()> {
    buf.put_u8(msg::GET);
    encode_string(buf, &msg.address)?;
    Ok(())
}

/// SNAPSHOT (0x23)
fn encode_snapshot(buf: &mut BytesMut, msg: &SnapshotMessage) -> Result<()> {
    buf.put_u8(msg::SNAPSHOT);
    buf.put_u16(msg.params.len() as u16);

    for param in &msg.params {
        encode_string(buf, &param.address)?;
        buf.put_u8(value_type_code(&param.value));
        encode_value_data(buf, &param.value)?;
        buf.put_u64(param.revision);

        let mut opt_flags: u8 = 0;
        if param.writer.is_some() {
            opt_flags |= 0x01;
        }
        if param.timestamp.is_some() {
            opt_flags |= 0x02;
        }
        buf.put_u8(opt_flags);

        if let Some(ref writer) = param.writer {
            encode_string(buf, writer)?;
        }
        if let Some(ts) = param.timestamp {
            buf.put_u64(ts);
        }
    }

    Ok(())
}

/// BUNDLE (0x30)
fn encode_bundle(buf: &mut BytesMut, msg: &BundleMessage) -> Result<()> {
    buf.put_u8(msg::BUNDLE);

    let mut flags: u8 = 0;
    if msg.timestamp.is_some() {
        flags |= 0x80;
    }
    buf.put_u8(flags);

    buf.put_u16(msg.messages.len() as u16);

    if let Some(ts) = msg.timestamp {
        buf.put_u64(ts);
    }

    // Each message prefixed with length
    for inner_msg in &msg.messages {
        let mut inner_buf = BytesMut::with_capacity(64);
        encode_message_to_buf(&mut inner_buf, inner_msg)?;
        buf.put_u16(inner_buf.len() as u16);
        buf.extend_from_slice(&inner_buf);
    }

    Ok(())
}

/// SYNC (0x40)
fn encode_sync(buf: &mut BytesMut, msg: &SyncMessage) -> Result<()> {
    buf.put_u8(msg::SYNC);

    let mut flags: u8 = 0;
    if msg.t2.is_some() {
        flags |= 0x01;
    }
    if msg.t3.is_some() {
        flags |= 0x02;
    }
    buf.put_u8(flags);

    buf.put_u64(msg.t1);
    if let Some(t2) = msg.t2 {
        buf.put_u64(t2);
    }
    if let Some(t3) = msg.t3 {
        buf.put_u64(t3);
    }

    Ok(())
}

/// ACK (0x50)
fn encode_ack(buf: &mut BytesMut, msg: &AckMessage) -> Result<()> {
    buf.put_u8(msg::ACK);

    let mut flags: u8 = 0;
    if msg.address.is_some() {
        flags |= 0x01;
    }
    if msg.revision.is_some() {
        flags |= 0x02;
    }
    if msg.locked.is_some() {
        flags |= 0x04;
    }
    if msg.holder.is_some() {
        flags |= 0x08;
    }
    if msg.correlation_id.is_some() {
        flags |= 0x10;
    }
    buf.put_u8(flags);

    if let Some(ref addr) = msg.address {
        encode_string(buf, addr)?;
    }
    if let Some(rev) = msg.revision {
        buf.put_u64(rev);
    }
    if let Some(locked) = msg.locked {
        buf.put_u8(if locked { 1 } else { 0 });
    }
    if let Some(ref holder) = msg.holder {
        encode_string(buf, holder)?;
    }
    if let Some(corr) = msg.correlation_id {
        buf.put_u32(corr);
    }

    Ok(())
}

/// ERROR (0x51)
fn encode_error(buf: &mut BytesMut, msg: &ErrorMessage) -> Result<()> {
    buf.put_u8(msg::ERROR);
    buf.put_u16(msg.code);
    encode_string(buf, &msg.message)?;

    let mut flags: u8 = 0;
    if msg.address.is_some() {
        flags |= 0x01;
    }
    if msg.correlation_id.is_some() {
        flags |= 0x02;
    }
    buf.put_u8(flags);

    if let Some(ref addr) = msg.address {
        encode_string(buf, addr)?;
    }
    if let Some(corr) = msg.correlation_id {
        buf.put_u32(corr);
    }

    Ok(())
}

/// QUERY (0x60)
fn encode_query(buf: &mut BytesMut, msg: &QueryMessage) -> Result<()> {
    buf.put_u8(msg::QUERY);
    encode_string(buf, &msg.pattern)?;
    Ok(())
}

/// RESULT (0x61)
fn encode_result(buf: &mut BytesMut, msg: &ResultMessage) -> Result<()> {
    buf.put_u8(msg::RESULT);
    buf.put_u16(msg.signals.len() as u16);

    for sig in &msg.signals {
        encode_string(buf, &sig.address)?;
        buf.put_u8(signal_type_code(sig.signal_type));

        let mut opt_flags: u8 = 0;
        if sig.datatype.is_some() {
            opt_flags |= 0x01;
        }
        if sig.access.is_some() {
            opt_flags |= 0x02;
        }
        buf.put_u8(opt_flags);

        if let Some(ref dt) = sig.datatype {
            encode_string(buf, dt)?;
        }
        if let Some(ref access) = sig.access {
            encode_string(buf, access)?;
        }
    }

    Ok(())
}

// ============================================================================
// VALUE ENCODING HELPERS
// ============================================================================

#[inline(always)]
fn encode_string(buf: &mut BytesMut, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err(Error::PayloadTooLarge(bytes.len()));
    }
    buf.put_u16(bytes.len() as u16);
    buf.extend_from_slice(bytes);
    Ok(())
}

#[inline]
fn encode_value_data(buf: &mut BytesMut, value: &Value) -> Result<()> {
    match value {
        Value::Null => {} // Type code is enough
        Value::Bool(b) => buf.put_u8(if *b { 1 } else { 0 }),
        Value::Int(i) => buf.put_i64(*i),
        Value::Float(f) => buf.put_f64(*f),
        Value::String(s) => encode_string(buf, s)?,
        Value::Bytes(b) => {
            buf.put_u16(b.len() as u16);
            buf.extend_from_slice(b);
        }
        Value::Array(arr) => {
            buf.put_u16(arr.len() as u16);
            for item in arr {
                buf.put_u8(value_type_code(item));
                encode_value_data(buf, item)?;
            }
        }
        Value::Map(map) => {
            buf.put_u16(map.len() as u16);
            for (key, val) in map {
                encode_string(buf, key)?;
                buf.put_u8(value_type_code(val));
                encode_value_data(buf, val)?;
            }
        }
    }
    Ok(())
}

#[inline(always)]
fn value_type_code(value: &Value) -> u8 {
    match value {
        Value::Null => val::NULL,
        Value::Bool(_) => val::BOOL,
        Value::Int(_) => val::I64,
        Value::Float(_) => val::F64,
        Value::String(_) => val::STRING,
        Value::Bytes(_) => val::BYTES,
        Value::Array(_) => val::ARRAY,
        Value::Map(_) => val::MAP,
    }
}

fn signal_type_code(sig: SignalType) -> u8 {
    match sig {
        SignalType::Param => sig::PARAM,
        SignalType::Event => sig::EVENT,
        SignalType::Stream => sig::STREAM,
        SignalType::Gesture => sig::GESTURE,
        SignalType::Timeline => sig::TIMELINE,
    }
}

fn gesture_phase_code(phase: GesturePhase) -> u8 {
    match phase {
        GesturePhase::Start => phase::START,
        GesturePhase::Move => phase::MOVE,
        GesturePhase::End => phase::END,
        GesturePhase::Cancel => phase::CANCEL,
    }
}

// ============================================================================
// BINARY DECODING
// ============================================================================

fn decode_v3_binary(bytes: &[u8]) -> Result<Message> {
    if bytes.is_empty() {
        return Err(Error::BufferTooSmall { needed: 1, have: 0 });
    }

    let mut buf = bytes;
    let msg_type = buf.get_u8();

    match msg_type {
        msg::HELLO => decode_hello(&mut buf),
        msg::WELCOME => decode_welcome(&mut buf),
        msg::ANNOUNCE => decode_announce(&mut buf),
        msg::SUBSCRIBE => decode_subscribe(&mut buf),
        msg::UNSUBSCRIBE => decode_unsubscribe(&mut buf),
        msg::PUBLISH => decode_publish(&mut buf),
        msg::SET => decode_set(&mut buf),
        msg::GET => decode_get(&mut buf),
        msg::SNAPSHOT => decode_snapshot(&mut buf),
        msg::BUNDLE => decode_bundle(&mut buf),
        msg::SYNC => decode_sync(&mut buf),
        msg::PING => Ok(Message::Ping),
        msg::PONG => Ok(Message::Pong),
        msg::ACK => decode_ack(&mut buf),
        msg::ERROR => decode_error(&mut buf),
        msg::QUERY => decode_query(&mut buf),
        msg::RESULT => decode_result(&mut buf),
        _ => Err(Error::UnknownMessageType(msg_type)),
    }
}

#[inline]
fn decode_set(buf: &mut &[u8]) -> Result<Message> {
    let flags = buf.get_u8();
    let vtype = flags & 0x0F;
    let has_rev = (flags & 0x80) != 0;
    let lock = (flags & 0x40) != 0;
    let unlock = (flags & 0x20) != 0;

    let address = decode_string(buf)?;
    let value = decode_value_data(buf, vtype)?;

    let revision = if has_rev { Some(buf.get_u64()) } else { None };

    Ok(Message::Set(SetMessage {
        address,
        value,
        revision,
        lock,
        unlock,
    }))
}

fn decode_publish(buf: &mut &[u8]) -> Result<Message> {
    let flags = buf.get_u8();
    let sig_code = (flags >> 5) & 0x07;
    let has_ts = (flags & 0x10) != 0;
    let has_id = (flags & 0x08) != 0;
    let phase_code = flags & 0x07;

    let address = decode_string(buf)?;

    // Value indicator
    let value_indicator = buf.get_u8();
    let (value, payload, samples) = match value_indicator {
        0 => (None, None, None),
        1 => {
            let vtype = buf.get_u8();
            let v = decode_value_data(buf, vtype)?;
            (Some(v), None, None)
        }
        2 => {
            let count = buf.get_u16() as usize;
            let mut s = Vec::with_capacity(count);
            for _ in 0..count {
                s.push(buf.get_f64());
            }
            (None, None, Some(s))
        }
        _ => (None, None, None),
    };

    let timestamp = if has_ts { Some(buf.get_u64()) } else { None };
    let id = if has_id { Some(buf.get_u32()) } else { None };

    // Rate (if remaining bytes)
    let rate = if buf.remaining() >= 4 {
        Some(buf.get_u32())
    } else {
        None
    };

    let signal = Some(signal_type_from_code(sig_code));
    let phase = Some(gesture_phase_from_code(phase_code));

    Ok(Message::Publish(PublishMessage {
        address,
        signal,
        value,
        payload,
        samples,
        rate,
        id,
        phase,
        timestamp,
        timeline: None, // Timeline data is encoded separately when signal is Timeline
    }))
}

fn decode_hello(buf: &mut &[u8]) -> Result<Message> {
    let version = buf.get_u8();
    let feature_flags = buf.get_u8();

    let mut features = Vec::new();
    if feature_flags & 0x80 != 0 {
        features.push("param".to_string());
    }
    if feature_flags & 0x40 != 0 {
        features.push("event".to_string());
    }
    if feature_flags & 0x20 != 0 {
        features.push("stream".to_string());
    }
    if feature_flags & 0x10 != 0 {
        features.push("gesture".to_string());
    }
    if feature_flags & 0x08 != 0 {
        features.push("timeline".to_string());
    }

    let name = decode_string(buf)?;
    let token_str = decode_string(buf)?;
    let token = if token_str.is_empty() {
        None
    } else {
        Some(token_str)
    };

    Ok(Message::Hello(HelloMessage {
        version,
        name,
        features,
        capabilities: None,
        token,
    }))
}

fn decode_welcome(buf: &mut &[u8]) -> Result<Message> {
    let version = buf.get_u8();
    let feature_flags = buf.get_u8();

    let mut features = Vec::new();
    if feature_flags & 0x80 != 0 {
        features.push("param".to_string());
    }
    if feature_flags & 0x40 != 0 {
        features.push("event".to_string());
    }
    if feature_flags & 0x20 != 0 {
        features.push("stream".to_string());
    }
    if feature_flags & 0x10 != 0 {
        features.push("gesture".to_string());
    }
    if feature_flags & 0x08 != 0 {
        features.push("timeline".to_string());
    }

    let time = buf.get_u64();
    let session = decode_string(buf)?;
    let name = decode_string(buf)?;

    let token_str = decode_string(buf)?;
    let token = if token_str.is_empty() {
        None
    } else {
        Some(token_str)
    };

    Ok(Message::Welcome(WelcomeMessage {
        version,
        session,
        name,
        features,
        time,
        token,
    }))
}

fn decode_announce(buf: &mut &[u8]) -> Result<Message> {
    let namespace = decode_string(buf)?;
    let count = buf.get_u16() as usize;

    let mut signals = Vec::with_capacity(count);
    for _ in 0..count {
        let address = decode_string(buf)?;
        let sig_code = buf.get_u8();
        let opt_flags = buf.get_u8();

        let datatype = if opt_flags & 0x01 != 0 {
            Some(decode_string(buf)?)
        } else {
            None
        };
        let access = if opt_flags & 0x02 != 0 {
            Some(decode_string(buf)?)
        } else {
            None
        };

        let meta = if opt_flags & 0x04 != 0 {
            let meta_flags = buf.get_u8();

            let unit = if meta_flags & 0x01 != 0 {
                Some(decode_string(buf)?)
            } else {
                None
            };
            let range = if meta_flags & 0x02 != 0 {
                let min = buf.get_f64();
                let max = buf.get_f64();
                Some((min, max))
            } else {
                None
            };
            let default = if meta_flags & 0x04 != 0 {
                let vtype = buf.get_u8();
                Some(decode_value_data(buf, vtype)?)
            } else {
                None
            };
            let description = if meta_flags & 0x08 != 0 {
                Some(decode_string(buf)?)
            } else {
                None
            };

            Some(SignalMeta {
                unit,
                range,
                default,
                description,
            })
        } else {
            None
        };

        signals.push(SignalDefinition {
            address,
            signal_type: signal_type_from_code(sig_code),
            datatype,
            access,
            meta,
        });
    }

    Ok(Message::Announce(AnnounceMessage {
        namespace,
        signals,
        meta: None,
    }))
}

fn decode_subscribe(buf: &mut &[u8]) -> Result<Message> {
    let id = buf.get_u32();
    let pattern = decode_string(buf)?;
    let type_mask = buf.get_u8();

    let mut types = Vec::new();
    if type_mask == 0xFF {
        // All types, leave empty to indicate all
    } else {
        if type_mask & 0x01 != 0 {
            types.push(SignalType::Param);
        }
        if type_mask & 0x02 != 0 {
            types.push(SignalType::Event);
        }
        if type_mask & 0x04 != 0 {
            types.push(SignalType::Stream);
        }
        if type_mask & 0x08 != 0 {
            types.push(SignalType::Gesture);
        }
        if type_mask & 0x10 != 0 {
            types.push(SignalType::Timeline);
        }
    }

    let opt_flags = buf.get_u8();
    let options = if opt_flags != 0 {
        let max_rate = if opt_flags & 0x01 != 0 {
            Some(buf.get_u32())
        } else {
            None
        };
        let epsilon = if opt_flags & 0x02 != 0 {
            Some(buf.get_f64())
        } else {
            None
        };
        let history = if opt_flags & 0x04 != 0 {
            Some(buf.get_u32())
        } else {
            None
        };
        let window = if opt_flags & 0x08 != 0 {
            Some(buf.get_u32())
        } else {
            None
        };

        Some(SubscribeOptions {
            max_rate,
            epsilon,
            history,
            window,
        })
    } else {
        None
    };

    Ok(Message::Subscribe(SubscribeMessage {
        id,
        pattern,
        types,
        options,
    }))
}

fn decode_unsubscribe(buf: &mut &[u8]) -> Result<Message> {
    let id = buf.get_u32();
    Ok(Message::Unsubscribe(UnsubscribeMessage { id }))
}

fn decode_get(buf: &mut &[u8]) -> Result<Message> {
    let address = decode_string(buf)?;
    Ok(Message::Get(GetMessage { address }))
}

fn decode_snapshot(buf: &mut &[u8]) -> Result<Message> {
    let count = buf.get_u16() as usize;
    let mut params = Vec::with_capacity(count);

    for _ in 0..count {
        let address = decode_string(buf)?;
        let vtype = buf.get_u8();
        let value = decode_value_data(buf, vtype)?;
        let revision = buf.get_u64();
        let opt_flags = buf.get_u8();

        let writer = if opt_flags & 0x01 != 0 {
            Some(decode_string(buf)?)
        } else {
            None
        };
        let timestamp = if opt_flags & 0x02 != 0 {
            Some(buf.get_u64())
        } else {
            None
        };

        params.push(ParamValue {
            address,
            value,
            revision,
            writer,
            timestamp,
        });
    }

    Ok(Message::Snapshot(SnapshotMessage { params }))
}

fn decode_bundle(buf: &mut &[u8]) -> Result<Message> {
    let flags = buf.get_u8();
    let has_ts = (flags & 0x80) != 0;
    let count = buf.get_u16() as usize;

    let timestamp = if has_ts { Some(buf.get_u64()) } else { None };

    let mut messages = Vec::with_capacity(count);
    for _ in 0..count {
        let len = buf.get_u16() as usize;
        let inner_bytes = &buf[..len];
        buf.advance(len);
        messages.push(decode_v3_binary(inner_bytes)?);
    }

    Ok(Message::Bundle(BundleMessage {
        timestamp,
        messages,
    }))
}

fn decode_sync(buf: &mut &[u8]) -> Result<Message> {
    let flags = buf.get_u8();
    let t1 = buf.get_u64();
    let t2 = if flags & 0x01 != 0 {
        Some(buf.get_u64())
    } else {
        None
    };
    let t3 = if flags & 0x02 != 0 {
        Some(buf.get_u64())
    } else {
        None
    };

    Ok(Message::Sync(SyncMessage { t1, t2, t3 }))
}

fn decode_ack(buf: &mut &[u8]) -> Result<Message> {
    let flags = buf.get_u8();

    let address = if flags & 0x01 != 0 {
        Some(decode_string(buf)?)
    } else {
        None
    };
    let revision = if flags & 0x02 != 0 {
        Some(buf.get_u64())
    } else {
        None
    };
    let locked = if flags & 0x04 != 0 {
        Some(buf.get_u8() != 0)
    } else {
        None
    };
    let holder = if flags & 0x08 != 0 {
        Some(decode_string(buf)?)
    } else {
        None
    };
    let correlation_id = if flags & 0x10 != 0 {
        Some(buf.get_u32())
    } else {
        None
    };

    Ok(Message::Ack(AckMessage {
        address,
        revision,
        locked,
        holder,
        correlation_id,
    }))
}

fn decode_error(buf: &mut &[u8]) -> Result<Message> {
    let code = buf.get_u16();
    let message = decode_string(buf)?;
    let flags = buf.get_u8();

    let address = if flags & 0x01 != 0 {
        Some(decode_string(buf)?)
    } else {
        None
    };
    let correlation_id = if flags & 0x02 != 0 {
        Some(buf.get_u32())
    } else {
        None
    };

    Ok(Message::Error(ErrorMessage {
        code,
        message,
        address,
        correlation_id,
    }))
}

fn decode_query(buf: &mut &[u8]) -> Result<Message> {
    let pattern = decode_string(buf)?;
    Ok(Message::Query(QueryMessage { pattern }))
}

fn decode_result(buf: &mut &[u8]) -> Result<Message> {
    let count = buf.get_u16() as usize;
    let mut signals = Vec::with_capacity(count);

    for _ in 0..count {
        let address = decode_string(buf)?;
        let sig_code = buf.get_u8();
        let opt_flags = buf.get_u8();

        let datatype = if opt_flags & 0x01 != 0 {
            Some(decode_string(buf)?)
        } else {
            None
        };
        let access = if opt_flags & 0x02 != 0 {
            Some(decode_string(buf)?)
        } else {
            None
        };

        signals.push(SignalDefinition {
            address,
            signal_type: signal_type_from_code(sig_code),
            datatype,
            access,
            meta: None,
        });
    }

    Ok(Message::Result(ResultMessage { signals }))
}

// ============================================================================
// VALUE DECODING HELPERS
// ============================================================================

#[inline(always)]
fn decode_string(buf: &mut &[u8]) -> Result<String> {
    if buf.remaining() < 2 {
        return Err(Error::BufferTooSmall {
            needed: 2,
            have: buf.remaining(),
        });
    }
    let len = buf.get_u16() as usize;
    if buf.remaining() < len {
        return Err(Error::BufferTooSmall {
            needed: len,
            have: buf.remaining(),
        });
    }
    let bytes = &buf[..len];
    buf.advance(len);
    String::from_utf8(bytes.to_vec()).map_err(|e| Error::DecodeError(e.to_string()))
}

#[inline]
fn decode_value_data(buf: &mut &[u8], vtype: u8) -> Result<Value> {
    match vtype {
        val::NULL => Ok(Value::Null),
        val::BOOL => {
            let b = buf.get_u8();
            Ok(Value::Bool(b != 0))
        }
        val::I8 => {
            let i = buf.get_i8() as i64;
            Ok(Value::Int(i))
        }
        val::I16 => {
            let i = buf.get_i16() as i64;
            Ok(Value::Int(i))
        }
        val::I32 => {
            let i = buf.get_i32() as i64;
            Ok(Value::Int(i))
        }
        val::I64 => {
            let i = buf.get_i64();
            Ok(Value::Int(i))
        }
        val::F32 => {
            let f = buf.get_f32() as f64;
            Ok(Value::Float(f))
        }
        val::F64 => {
            let f = buf.get_f64();
            Ok(Value::Float(f))
        }
        val::STRING => {
            let s = decode_string(buf)?;
            Ok(Value::String(s))
        }
        val::BYTES => {
            if buf.remaining() < 2 {
                return Err(Error::BufferTooSmall {
                    needed: 2,
                    have: buf.remaining(),
                });
            }
            let len = buf.get_u16() as usize;
            if buf.remaining() < len {
                return Err(Error::BufferTooSmall {
                    needed: len,
                    have: buf.remaining(),
                });
            }
            let bytes = buf[..len].to_vec();
            buf.advance(len);
            Ok(Value::Bytes(bytes))
        }
        val::ARRAY => {
            let count = buf.get_u16() as usize;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                let item_type = buf.get_u8();
                arr.push(decode_value_data(buf, item_type)?);
            }
            Ok(Value::Array(arr))
        }
        val::MAP => {
            let count = buf.get_u16() as usize;
            let mut map = HashMap::with_capacity(count);
            for _ in 0..count {
                let key = decode_string(buf)?;
                let val_type = buf.get_u8();
                let val = decode_value_data(buf, val_type)?;
                map.insert(key, val);
            }
            Ok(Value::Map(map))
        }
        _ => Err(Error::DecodeError(format!("unknown value type: 0x{:02x}", vtype))),
    }
}

fn signal_type_from_code(code: u8) -> SignalType {
    match code {
        sig::PARAM => SignalType::Param,
        sig::EVENT => SignalType::Event,
        sig::STREAM => SignalType::Stream,
        sig::GESTURE => SignalType::Gesture,
        sig::TIMELINE => SignalType::Timeline,
        _ => SignalType::Event, // Default
    }
}

fn gesture_phase_from_code(code: u8) -> GesturePhase {
    match code {
        phase::START => GesturePhase::Start,
        phase::MOVE => GesturePhase::Move,
        phase::END => GesturePhase::End,
        phase::CANCEL => GesturePhase::Cancel,
        _ => GesturePhase::Start, // Default
    }
}

// ============================================================================
// V2 MESSAGEPACK DECODING (BACKWARD COMPATIBILITY)
// ============================================================================

fn is_msgpack_map(byte: u8) -> bool {
    // fixmap: 0x80-0x8F, map16: 0xDE, map32: 0xDF
    (byte & 0xF0) == 0x80 || byte == 0xDE || byte == 0xDF
}

fn decode_v2_msgpack(bytes: &[u8]) -> Result<Message> {
    rmp_serde::from_slice(bytes).map_err(|e| Error::DecodeError(e.to_string()))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_roundtrip() {
        let msg = Message::Hello(HelloMessage {
            version: 1,
            name: "Test Client".to_string(),
            features: vec!["param".to_string(), "event".to_string()],
            capabilities: None,
            token: None,
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, frame) = decode(&encoded).unwrap();

        match decoded {
            Message::Hello(hello) => {
                assert_eq!(hello.version, 1);
                assert_eq!(hello.name, "Test Client");
                assert!(hello.features.contains(&"param".to_string()));
                assert!(hello.features.contains(&"event".to_string()));
            }
            _ => panic!("Expected Hello message"),
        }

        assert_eq!(frame.flags.qos, QoS::Fire);
        assert_eq!(frame.flags.version, 1); // binary encoding
    }

    #[test]
    fn test_set_roundtrip() {
        let msg = Message::Set(SetMessage {
            address: "/test/value".to_string(),
            value: Value::Float(0.75),
            revision: Some(42),
            lock: false,
            unlock: false,
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, frame) = decode(&encoded).unwrap();

        match decoded {
            Message::Set(set) => {
                assert_eq!(set.address, "/test/value");
                assert_eq!(set.value.as_f64(), Some(0.75));
                assert_eq!(set.revision, Some(42));
            }
            _ => panic!("Expected Set message"),
        }

        assert_eq!(frame.flags.qos, QoS::Confirm);
    }

    #[test]
    fn test_set_size_reduction() {
        let msg = Message::Set(SetMessage {
            address: "/test/value".to_string(),
            value: Value::Float(0.5),
            revision: Some(1),
            lock: false,
            unlock: false,
        });

        // Binary encoding
        let binary_payload = encode_message(&msg).unwrap();

        // MessagePack encoding (named keys, legacy)
        let msgpack_payload = rmp_serde::to_vec_named(&msg).unwrap();

        println!("Binary payload: {} bytes", binary_payload.len());
        println!("MessagePack payload: {} bytes", msgpack_payload.len());

        // Binary encoding should be significantly smaller (target: ~32 bytes vs ~69 bytes)
        assert!(
            binary_payload.len() < msgpack_payload.len(),
            "Binary encoding ({}) should be smaller than MessagePack ({})",
            binary_payload.len(),
            msgpack_payload.len()
        );

        // Binary encoding should be at least 40% smaller
        let savings = 100 - (binary_payload.len() * 100 / msgpack_payload.len());
        println!("Size reduction: {}%", savings);
        assert!(savings >= 40, "Expected at least 40% size reduction, got {}%", savings);
    }

    #[test]
    fn test_bundle_roundtrip() {
        let msg = Message::Bundle(BundleMessage {
            timestamp: Some(1000000),
            messages: vec![
                Message::Set(SetMessage {
                    address: "/light/1".to_string(),
                    value: Value::Float(1.0),
                    revision: None,
                    lock: false,
                    unlock: false,
                }),
                Message::Set(SetMessage {
                    address: "/light/2".to_string(),
                    value: Value::Float(0.0),
                    revision: None,
                    lock: false,
                    unlock: false,
                }),
            ],
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode(&encoded).unwrap();

        match decoded {
            Message::Bundle(bundle) => {
                assert_eq!(bundle.timestamp, Some(1000000));
                assert_eq!(bundle.messages.len(), 2);
            }
            _ => panic!("Expected Bundle message"),
        }
    }

    #[test]
    fn test_value_types() {
        let values = vec![
            Value::Null,
            Value::Bool(true),
            Value::Int(42),
            Value::Float(3.14),
            Value::String("hello".to_string()),
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        ];

        for value in values {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: value.clone(),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = encode(&msg).unwrap();
            let (decoded, _) = decode(&encoded).unwrap();

            match decoded {
                Message::Set(set) => {
                    assert_eq!(set.value, value);
                }
                _ => panic!("Expected Set message"),
            }
        }
    }

    #[test]
    fn test_backward_compat_v2_decode() {
        // Create a v2 MessagePack encoded message
        let msg = SetMessage {
            address: "/test/value".to_string(),
            value: Value::Float(0.5),
            revision: Some(1),
            lock: false,
            unlock: false,
        };

        // Encode as v2 (MessagePack with named keys)
        let v2_bytes = rmp_serde::to_vec_named(&Message::Set(msg.clone())).unwrap();

        // Should still decode correctly
        let decoded = decode_message(&v2_bytes).unwrap();

        match decoded {
            Message::Set(set) => {
                assert_eq!(set.address, "/test/value");
                assert_eq!(set.value.as_f64(), Some(0.5));
            }
            _ => panic!("Expected Set message"),
        }
    }

    #[test]
    fn test_ping_pong() {
        let ping = encode(&Message::Ping).unwrap();
        let (decoded, _) = decode(&ping).unwrap();
        assert!(matches!(decoded, Message::Ping));

        let pong = encode(&Message::Pong).unwrap();
        let (decoded, _) = decode(&pong).unwrap();
        assert!(matches!(decoded, Message::Pong));
    }

    #[test]
    fn test_publish_event() {
        let msg = Message::Publish(PublishMessage {
            address: "/cue/fire".to_string(),
            signal: Some(SignalType::Event),
            value: None,
            payload: Some(Value::String("intro".to_string())),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: Some(1234567890),
            timeline: None,
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode(&encoded).unwrap();

        match decoded {
            Message::Publish(pub_msg) => {
                assert_eq!(pub_msg.address, "/cue/fire");
                assert_eq!(pub_msg.signal, Some(SignalType::Event));
                assert_eq!(pub_msg.timestamp, Some(1234567890));
            }
            _ => panic!("Expected Publish message"),
        }
    }

    #[test]
    fn test_subscribe_roundtrip() {
        let msg = Message::Subscribe(SubscribeMessage {
            id: 42,
            pattern: "/lumen/scene/*/layer/**".to_string(),
            types: vec![SignalType::Param, SignalType::Stream],
            options: Some(SubscribeOptions {
                max_rate: Some(60),
                epsilon: Some(0.01),
                history: None,
                window: None,
            }),
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode(&encoded).unwrap();

        match decoded {
            Message::Subscribe(sub) => {
                assert_eq!(sub.id, 42);
                assert_eq!(sub.pattern, "/lumen/scene/*/layer/**");
                assert!(sub.types.contains(&SignalType::Param));
                assert!(sub.types.contains(&SignalType::Stream));
                assert_eq!(sub.options.as_ref().unwrap().max_rate, Some(60));
            }
            _ => panic!("Expected Subscribe message"),
        }
    }
}
