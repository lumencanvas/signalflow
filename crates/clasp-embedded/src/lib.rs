//! CLASP Embedded
//!
//! Minimal `no_std` implementation of the **standard CLASP binary protocol**.
//!
//! This crate provides both client AND server (mini-router) capabilities
//! for embedded devices like ESP32, Raspberry Pi Pico, etc.
//!
//! # Protocol Compatibility
//!
//! **Uses the same compact binary protocol as the full CLASP implementation.**
//! Messages from embedded devices are fully compatible with desktop/cloud routers.
//!
//! # Memory Budget
//!
//! | Component | ESP32 (320KB) | RP2040 (264KB) | Notes |
//! |-----------|---------------|----------------|-------|
//! | Client | ~2KB | ~2KB | State cache, subscriptions |
//! | Server | ~4KB | ~4KB | + session management |
//! | Buffers | ~1KB | ~1KB | TX/RX configurable |
//!
//! # Features
//!
//! - `alloc` - Enable heap allocation for dynamic strings (recommended for ESP32)
//! - `server` - Enable mini-router/server mode
//! - `client` - Enable client mode (default)

#![no_std]
#![allow(dead_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

// ============================================================================
// CLASP Protocol Constants (same as clasp-core)
// ============================================================================

/// Protocol magic byte
pub const MAGIC: u8 = 0x53; // 'S' for Stream

/// Protocol version (used in HELLO messages)
pub const VERSION: u8 = 1;

/// Message type codes (standard CLASP binary format)
pub mod msg {
    pub const HELLO: u8 = 0x01;
    pub const WELCOME: u8 = 0x02;
    pub const SUBSCRIBE: u8 = 0x10;
    pub const UNSUBSCRIBE: u8 = 0x11;
    pub const PUBLISH: u8 = 0x20;
    pub const SET: u8 = 0x21;
    pub const GET: u8 = 0x22;
    pub const SNAPSHOT: u8 = 0x23;
    pub const PING: u8 = 0x41;
    pub const PONG: u8 = 0x42;
    pub const ACK: u8 = 0x50;
    pub const ERROR: u8 = 0x51;
}

/// Value type codes (standard CLASP binary format)
pub mod val {
    pub const NULL: u8 = 0x00;
    pub const BOOL: u8 = 0x01;
    pub const I32: u8 = 0x04;
    pub const I64: u8 = 0x05;
    pub const F32: u8 = 0x06;
    pub const F64: u8 = 0x07;
    pub const STRING: u8 = 0x08;
    pub const BYTES: u8 = 0x09;
}

// ============================================================================
// Frame Format (standard CLASP binary format)
// ============================================================================

/// Frame header size (without timestamp)
pub const HEADER_SIZE: usize = 4;

/// Maximum payload for embedded (configurable, smaller than full 65535)
pub const MAX_PAYLOAD: usize = 1024;

/// Decode frame header, returns (flags, payload_len) or None
pub fn decode_header(buf: &[u8]) -> Option<(u8, usize)> {
    if buf.len() < HEADER_SIZE || buf[0] != MAGIC {
        return None;
    }
    let flags = buf[1];
    let len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    Some((flags, len))
}

/// Frame flags for compact binary encoding
/// Bits: [qos:2][has_ts:1][enc:1][cmp:1][rsv:1][version:2]
pub const FLAGS_BINARY: u8 = 0x01; // version=1 (compact binary), rest default

/// Encode frame header with binary encoding flags
pub fn encode_header(buf: &mut [u8], _flags: u8, payload_len: usize) -> usize {
    if buf.len() < HEADER_SIZE {
        return 0;
    }
    buf[0] = MAGIC;
    buf[1] = FLAGS_BINARY; // Always use compact binary encoding
    let len = (payload_len as u16).to_be_bytes();
    buf[2] = len[0];
    buf[3] = len[1];
    HEADER_SIZE
}

// ============================================================================
// Value Encoding/Decoding (compact binary format)
// ============================================================================

/// Simple value type for embedded
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
}

impl Value {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Encode a value, returns bytes written
pub fn encode_value(buf: &mut [u8], value: &Value) -> usize {
    match value {
        Value::Null => {
            if buf.is_empty() {
                return 0;
            }
            buf[0] = val::NULL;
            1
        }
        Value::Bool(b) => {
            if buf.len() < 2 {
                return 0;
            }
            buf[0] = val::BOOL;
            buf[1] = if *b { 1 } else { 0 };
            2
        }
        Value::Int(i) => {
            if buf.len() < 9 {
                return 0;
            }
            buf[0] = val::I64;
            buf[1..9].copy_from_slice(&i.to_be_bytes());
            9
        }
        Value::Float(f) => {
            if buf.len() < 9 {
                return 0;
            }
            buf[0] = val::F64;
            buf[1..9].copy_from_slice(&f.to_be_bytes());
            9
        }
    }
}

/// Decode a value, returns (value, bytes_consumed)
pub fn decode_value(buf: &[u8]) -> Option<(Value, usize)> {
    if buf.is_empty() {
        return None;
    }
    match buf[0] {
        val::NULL => Some((Value::Null, 1)),
        val::BOOL => {
            if buf.len() < 2 {
                return None;
            }
            Some((Value::Bool(buf[1] != 0), 2))
        }
        val::I32 => {
            if buf.len() < 5 {
                return None;
            }
            let i = i32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]);
            Some((Value::Int(i as i64), 5))
        }
        val::I64 => {
            if buf.len() < 9 {
                return None;
            }
            let i = i64::from_be_bytes([
                buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
            ]);
            Some((Value::Int(i), 9))
        }
        val::F32 => {
            if buf.len() < 5 {
                return None;
            }
            let f = f32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]);
            Some((Value::Float(f as f64), 5))
        }
        val::F64 => {
            if buf.len() < 9 {
                return None;
            }
            let f = f64::from_be_bytes([
                buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
            ]);
            Some((Value::Float(f), 9))
        }
        _ => None,
    }
}

// ============================================================================
// String Encoding (length-prefixed, standard CLASP format)
// ============================================================================

/// Encode a string (u16 length prefix)
pub fn encode_string(buf: &mut [u8], s: &str) -> usize {
    let bytes = s.as_bytes();
    if buf.len() < 2 + bytes.len() {
        return 0;
    }
    let len = (bytes.len() as u16).to_be_bytes();
    buf[0] = len[0];
    buf[1] = len[1];
    buf[2..2 + bytes.len()].copy_from_slice(bytes);
    2 + bytes.len()
}

/// Decode a string, returns (str slice, bytes consumed)
pub fn decode_string(buf: &[u8]) -> Option<(&str, usize)> {
    if buf.len() < 2 {
        return None;
    }
    let len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
    if buf.len() < 2 + len {
        return None;
    }
    let s = core::str::from_utf8(&buf[2..2 + len]).ok()?;
    Some((s, 2 + len))
}

// ============================================================================
// Message Encoding (compact binary format)
// ============================================================================

/// Get value type code for flags byte
fn value_type_code(value: &Value) -> u8 {
    match value {
        Value::Null => val::NULL,
        Value::Bool(_) => val::BOOL,
        Value::Int(_) => val::I64,
        Value::Float(_) => val::F64,
    }
}

/// Encode value data only (without type byte, for SET messages)
fn encode_value_data(buf: &mut [u8], value: &Value) -> usize {
    match value {
        Value::Null => 0,
        Value::Bool(b) => {
            if buf.is_empty() {
                return 0;
            }
            buf[0] = if *b { 1 } else { 0 };
            1
        }
        Value::Int(i) => {
            if buf.len() < 8 {
                return 0;
            }
            buf[..8].copy_from_slice(&i.to_be_bytes());
            8
        }
        Value::Float(f) => {
            if buf.len() < 8 {
                return 0;
            }
            buf[..8].copy_from_slice(&f.to_be_bytes());
            8
        }
    }
}

/// Encode a SET message payload (without frame header)
/// Format: msg_type(1) + flags(1) + addr_len(2) + addr + value_data
/// Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
pub fn encode_set(buf: &mut [u8], address: &str, value: &Value) -> usize {
    if buf.len() < 2 {
        return 0;
    }

    // Message type
    buf[0] = msg::SET;

    // Flags: value type in lower 4 bits, no revision/lock/unlock
    let vtype = value_type_code(value);
    buf[1] = vtype & 0x0F;

    let mut offset = 2;

    // Address (length-prefixed)
    offset += encode_string(&mut buf[offset..], address);

    // Value data only (type is in flags)
    offset += encode_value_data(&mut buf[offset..], value);

    offset
}

/// Encode a complete SET frame (header + payload)
pub fn encode_set_frame(buf: &mut [u8], address: &str, value: &Value) -> usize {
    let header_size = HEADER_SIZE;
    let payload_start = header_size;

    let payload_len = encode_set(&mut buf[payload_start..], address, value);
    if payload_len == 0 {
        return 0;
    }

    encode_header(buf, 0, payload_len);
    header_size + payload_len
}

/// Encode a SUBSCRIBE message
pub fn encode_subscribe(buf: &mut [u8], pattern: &str) -> usize {
    if buf.is_empty() {
        return 0;
    }
    buf[0] = msg::SUBSCRIBE;
    let mut offset = 1;

    // subscription id (u32)
    if buf.len() < offset + 4 {
        return 0;
    }
    buf[offset..offset + 4].copy_from_slice(&0u32.to_be_bytes());
    offset += 4;

    // pattern
    offset += encode_string(&mut buf[offset..], pattern);

    // signal types count (0 = all)
    if buf.len() > offset {
        buf[offset] = 0;
        offset += 1;
    }

    offset
}

/// Encode a SUBSCRIBE frame
pub fn encode_subscribe_frame(buf: &mut [u8], pattern: &str) -> usize {
    let header_size = HEADER_SIZE;
    let payload_len = encode_subscribe(&mut buf[header_size..], pattern);
    if payload_len == 0 {
        return 0;
    }
    encode_header(buf, 0, payload_len);
    header_size + payload_len
}

/// Encode a HELLO message (binary format)
/// Format: msg_type(1) + version(1) + features(1) + name + token
pub fn encode_hello(buf: &mut [u8], name: &str) -> usize {
    if buf.len() < 6 {
        return 0;
    }

    // Message type
    buf[0] = msg::HELLO;

    // Protocol version
    buf[1] = VERSION;

    // Feature flags (all features supported)
    buf[2] = 0xF8; // param|event|stream|gesture|timeline

    let mut offset = 3;

    // Name
    offset += encode_string(&mut buf[offset..], name);

    // Token (none)
    if buf.len() >= offset + 2 {
        buf[offset] = 0;
        buf[offset + 1] = 0;
        offset += 2;
    }

    offset
}

/// Encode a HELLO frame
pub fn encode_hello_frame(buf: &mut [u8], name: &str) -> usize {
    let header_size = HEADER_SIZE;
    let payload_len = encode_hello(&mut buf[header_size..], name);
    if payload_len == 0 {
        return 0;
    }
    encode_header(buf, 0, payload_len);
    header_size + payload_len
}

/// Encode a PING frame
pub fn encode_ping_frame(buf: &mut [u8]) -> usize {
    if buf.len() < HEADER_SIZE + 1 {
        return 0;
    }
    encode_header(buf, 0, 1);
    buf[HEADER_SIZE] = msg::PING;
    HEADER_SIZE + 1
}

/// Encode a PONG frame
pub fn encode_pong_frame(buf: &mut [u8]) -> usize {
    if buf.len() < HEADER_SIZE + 1 {
        return 0;
    }
    encode_header(buf, 0, 1);
    buf[HEADER_SIZE] = msg::PONG;
    HEADER_SIZE + 1
}

// ============================================================================
// Message Decoding
// ============================================================================

/// Decoded message (zero-copy where possible)
#[derive(Debug)]
pub enum Message<'a> {
    Hello { name: &'a str, version: u8 },
    Welcome { session: &'a str },
    Set { address: &'a str, value: Value },
    Subscribe { id: u32, pattern: &'a str },
    Unsubscribe { id: u32 },
    Ping,
    Pong,
    Error { code: u16, message: &'a str },
    Unknown(u8),
}

/// Decode a message from a frame payload
pub fn decode_message(payload: &[u8]) -> Option<Message<'_>> {
    if payload.is_empty() {
        return None;
    }

    let msg_type = payload[0];
    let data = &payload[1..];

    match msg_type {
        msg::HELLO => {
            // HELLO format: version(1) + features(1) + name + token
            if data.len() < 2 {
                return None;
            }
            let version = data[0];
            let _features = data[1];
            let (name, _) = decode_string(&data[2..])?;
            Some(Message::Hello { name, version })
        }
        msg::WELCOME => {
            // WELCOME format: version(1) + features(1) + time(8) + session + name
            if data.len() < 10 {
                return None;
            }
            let _version = data[0];
            let _features = data[1];
            let _time = u64::from_be_bytes([
                data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
            ]);
            let (session, _) = decode_string(&data[10..])?;
            Some(Message::Welcome { session })
        }
        msg::SET => {
            // SET format: flags(1) + address + value_data
            // Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
            if data.is_empty() {
                return None;
            }
            let flags = data[0];
            let vtype = flags & 0x0F;
            let _has_rev = (flags & 0x80) != 0;

            let (address, offset) = decode_string(&data[1..])?;
            let value_data = &data[1 + offset..];

            let value = match vtype {
                val::NULL => Value::Null,
                val::BOOL => {
                    if value_data.is_empty() {
                        return None;
                    }
                    Value::Bool(value_data[0] != 0)
                }
                val::I64 => {
                    if value_data.len() < 8 {
                        return None;
                    }
                    let i = i64::from_be_bytes([
                        value_data[0],
                        value_data[1],
                        value_data[2],
                        value_data[3],
                        value_data[4],
                        value_data[5],
                        value_data[6],
                        value_data[7],
                    ]);
                    Value::Int(i)
                }
                val::F64 => {
                    if value_data.len() < 8 {
                        return None;
                    }
                    let f = f64::from_be_bytes([
                        value_data[0],
                        value_data[1],
                        value_data[2],
                        value_data[3],
                        value_data[4],
                        value_data[5],
                        value_data[6],
                        value_data[7],
                    ]);
                    Value::Float(f)
                }
                _ => return None, // Unsupported type
            };

            Some(Message::Set { address, value })
        }
        msg::SUBSCRIBE => {
            // SUBSCRIBE format: id(4) + pattern
            if data.len() < 4 {
                return None;
            }
            let id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            let (pattern, _) = decode_string(&data[4..])?;
            Some(Message::Subscribe { id, pattern })
        }
        msg::UNSUBSCRIBE => {
            // UNSUBSCRIBE format: id(4)
            if data.len() < 4 {
                return None;
            }
            let id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            Some(Message::Unsubscribe { id })
        }
        msg::PING => Some(Message::Ping),
        msg::PONG => Some(Message::Pong),
        msg::ERROR => {
            if data.len() < 2 {
                return None;
            }
            let code = u16::from_be_bytes([data[0], data[1]]);
            let (message, _) = decode_string(&data[2..]).unwrap_or(("", 0));
            Some(Message::Error { code, message })
        }
        _ => Some(Message::Unknown(msg_type)),
    }
}

// ============================================================================
// State Cache (Fixed Size, No Heap)
// ============================================================================

/// Maximum cached parameters
pub const MAX_CACHE_ENTRIES: usize = 32;

/// Maximum address length
pub const MAX_ADDRESS_LEN: usize = 64;

/// A cached parameter entry
#[derive(Clone)]
pub struct CacheEntry {
    address: [u8; MAX_ADDRESS_LEN],
    address_len: u8,
    value: Value,
    valid: bool,
}

impl Default for CacheEntry {
    fn default() -> Self {
        Self {
            address: [0; MAX_ADDRESS_LEN],
            address_len: 0,
            value: Value::Null,
            valid: false,
        }
    }
}

impl CacheEntry {
    fn address(&self) -> &str {
        core::str::from_utf8(&self.address[..self.address_len as usize]).unwrap_or("")
    }

    fn set_address(&mut self, addr: &str) {
        let bytes = addr.as_bytes();
        let len = bytes.len().min(MAX_ADDRESS_LEN);
        self.address[..len].copy_from_slice(&bytes[..len]);
        self.address_len = len as u8;
    }
}

/// Fixed-size parameter cache
pub struct StateCache {
    entries: [CacheEntry; MAX_CACHE_ENTRIES],
    count: usize,
}

impl StateCache {
    pub const fn new() -> Self {
        Self {
            entries: [const {
                CacheEntry {
                    address: [0; MAX_ADDRESS_LEN],
                    address_len: 0,
                    value: Value::Null,
                    valid: false,
                }
            }; MAX_CACHE_ENTRIES],
            count: 0,
        }
    }

    /// Get a cached value
    pub fn get(&self, address: &str) -> Option<Value> {
        for entry in &self.entries[..self.count] {
            if entry.valid && entry.address() == address {
                return Some(entry.value);
            }
        }
        None
    }

    /// Set a cached value
    pub fn set(&mut self, address: &str, value: Value) -> bool {
        // Update existing
        for entry in &mut self.entries[..self.count] {
            if entry.valid && entry.address() == address {
                entry.value = value;
                return true;
            }
        }

        // Add new
        if self.count < MAX_CACHE_ENTRIES {
            self.entries[self.count].set_address(address);
            self.entries[self.count].value = value;
            self.entries[self.count].valid = true;
            self.count += 1;
            return true;
        }

        false
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.valid = false;
        }
        self.count = 0;
    }
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Client (Compact Binary Protocol)
// ============================================================================

/// Client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
}

/// Buffer size for messages
pub const TX_BUF_SIZE: usize = 256;
pub const RX_BUF_SIZE: usize = 512;

/// Embedded CLASP client (compact binary protocol)
///
/// # Memory Usage
/// ~3KB total (cache + buffers + state)
pub struct Client {
    pub state: ClientState,
    pub cache: StateCache,
    tx_buf: [u8; TX_BUF_SIZE],
    rx_buf: [u8; RX_BUF_SIZE],
}

impl Client {
    pub const fn new() -> Self {
        Self {
            state: ClientState::Disconnected,
            cache: StateCache::new(),
            tx_buf: [0; TX_BUF_SIZE],
            rx_buf: [0; RX_BUF_SIZE],
        }
    }

    /// Prepare HELLO frame
    pub fn prepare_hello(&mut self, name: &str) -> &[u8] {
        let n = encode_hello_frame(&mut self.tx_buf, name);
        &self.tx_buf[..n]
    }

    /// Prepare SET frame
    pub fn prepare_set(&mut self, address: &str, value: Value) -> &[u8] {
        let n = encode_set_frame(&mut self.tx_buf, address, &value);
        &self.tx_buf[..n]
    }

    /// Prepare SUBSCRIBE frame
    pub fn prepare_subscribe(&mut self, pattern: &str) -> &[u8] {
        let n = encode_subscribe_frame(&mut self.tx_buf, pattern);
        &self.tx_buf[..n]
    }

    /// Prepare PING frame
    pub fn prepare_ping(&mut self) -> &[u8] {
        let n = encode_ping_frame(&mut self.tx_buf);
        &self.tx_buf[..n]
    }

    /// Process received frame data
    pub fn process<'a>(&mut self, data: &'a [u8]) -> Option<Message<'a>> {
        let (_, payload_len) = decode_header(data)?;
        let payload = &data[HEADER_SIZE..HEADER_SIZE + payload_len];
        let msg = decode_message(payload)?;

        match &msg {
            Message::Welcome { .. } => {
                self.state = ClientState::Connected;
            }
            Message::Set { address, value } => {
                self.cache.set(address, *value);
            }
            _ => {}
        }

        Some(msg)
    }

    pub fn is_connected(&self) -> bool {
        self.state == ClientState::Connected
    }

    pub fn get_cached(&self, address: &str) -> Option<Value> {
        self.cache.get(address)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Mini-Router/Server (Compact Binary Protocol)
// ============================================================================

#[cfg(feature = "server")]
pub mod server {
    use super::*;

    /// Maximum clients for embedded router
    pub const MAX_CLIENTS: usize = 4;

    /// Maximum subscriptions per client
    pub const MAX_SUBS_PER_CLIENT: usize = 8;

    /// Maximum pattern length for subscriptions
    pub const MAX_PATTERN_LEN: usize = 64;

    /// Subscription entry
    #[derive(Clone)]
    pub struct Subscription {
        pub active: bool,
        pub id: u32,
        pub pattern: [u8; MAX_PATTERN_LEN],
        pub pattern_len: usize,
    }

    impl Subscription {
        pub const fn empty() -> Self {
            Self {
                active: false,
                id: 0,
                pattern: [0; MAX_PATTERN_LEN],
                pattern_len: 0,
            }
        }

        /// Check if address matches this subscription pattern
        pub fn matches(&self, address: &str) -> bool {
            if !self.active || self.pattern_len == 0 {
                return false;
            }

            // Get pattern as &str
            let pattern = match core::str::from_utf8(&self.pattern[..self.pattern_len]) {
                Ok(s) => s,
                Err(_) => return false,
            };

            // Simple wildcard matching
            // * matches one segment, ** matches any number of segments
            Self::match_pattern(pattern, address)
        }

        fn match_pattern(pattern: &str, address: &str) -> bool {
            // Simple iterative matching without heap allocation
            // Split into segments manually
            let mut pattern_iter = pattern.split('/').filter(|s| !s.is_empty());
            let mut address_iter = address.split('/').filter(|s| !s.is_empty());

            loop {
                match (pattern_iter.next(), address_iter.next()) {
                    (None, None) => return true,
                    (Some("**"), _) => {
                        // ** matches zero or more segments
                        // Check if there's more pattern after **
                        if let Some(next_pattern) = pattern_iter.next() {
                            // Must find next_pattern in remaining address
                            loop {
                                match address_iter.next() {
                                    None => return next_pattern == "**",
                                    Some(seg) if seg == next_pattern || next_pattern == "*" => {
                                        // Continue matching rest
                                        break;
                                    }
                                    Some(_) => continue,
                                }
                            }
                        } else {
                            // ** at end matches everything
                            return true;
                        }
                    }
                    (Some("*"), Some(_)) => continue,
                    (Some(p), Some(a)) if p == a => continue,
                    (None, Some(_)) => return false,
                    (Some(_), None) => {
                        // Check if remaining pattern is just **
                        return pattern_iter.all(|p| p == "**");
                    }
                    _ => return false,
                }
            }
        }
    }

    /// Client session with subscriptions
    pub struct Session {
        pub active: bool,
        pub id: u8,
        pub subscriptions: [Subscription; MAX_SUBS_PER_CLIENT],
        pub sub_count: u8,
    }

    impl Session {
        pub const fn new() -> Self {
            Self {
                active: false,
                id: 0,
                subscriptions: [const { Subscription::empty() }; MAX_SUBS_PER_CLIENT],
                sub_count: 0,
            }
        }

        /// Add a subscription
        pub fn subscribe(&mut self, id: u32, pattern: &str) -> bool {
            if self.sub_count as usize >= MAX_SUBS_PER_CLIENT {
                return false;
            }
            if pattern.len() > MAX_PATTERN_LEN {
                return false;
            }

            // Find empty slot
            for sub in &mut self.subscriptions {
                if !sub.active {
                    sub.active = true;
                    sub.id = id;
                    sub.pattern[..pattern.len()].copy_from_slice(pattern.as_bytes());
                    sub.pattern_len = pattern.len();
                    self.sub_count += 1;
                    return true;
                }
            }
            false
        }

        /// Remove a subscription by ID
        pub fn unsubscribe(&mut self, id: u32) -> bool {
            for sub in &mut self.subscriptions {
                if sub.active && sub.id == id {
                    sub.active = false;
                    sub.pattern_len = 0;
                    self.sub_count = self.sub_count.saturating_sub(1);
                    return true;
                }
            }
            false
        }

        /// Check if any subscription matches the address
        pub fn has_match(&self, address: &str) -> bool {
            self.subscriptions.iter().any(|s| s.matches(address))
        }
    }

    /// Broadcast result - which clients should receive a message
    pub struct BroadcastList {
        pub clients: [bool; MAX_CLIENTS],
        pub count: u8,
    }

    impl BroadcastList {
        pub const fn empty() -> Self {
            Self {
                clients: [false; MAX_CLIENTS],
                count: 0,
            }
        }
    }

    /// Minimal embedded router with subscription support
    ///
    /// Can act as a local hub for sensors/actuators, forwarding to a main router.
    pub struct MiniRouter {
        pub state: StateCache,
        sessions: [Session; MAX_CLIENTS],
        session_count: u8,
        tx_buf: [u8; TX_BUF_SIZE],
    }

    impl MiniRouter {
        pub const fn new() -> Self {
            Self {
                state: StateCache::new(),
                sessions: [const { Session::new() }; MAX_CLIENTS],
                session_count: 0,
                tx_buf: [0; TX_BUF_SIZE],
            }
        }

        /// Process incoming message from a client
        ///
        /// Returns a response frame to send back to the client (if any)
        pub fn process(&mut self, client_id: u8, data: &[u8]) -> Option<&[u8]> {
            let (_, payload_len) = decode_header(data)?;
            let payload = &data[HEADER_SIZE..HEADER_SIZE + payload_len];
            let msg = decode_message(payload)?;

            match msg {
                Message::Hello { name, .. } => {
                    self.create_session(client_id);
                    Some(self.prepare_welcome(client_id))
                }
                Message::Subscribe { id, pattern } => {
                    self.handle_subscribe(client_id, id, pattern);
                    None // ACK could be sent
                }
                Message::Unsubscribe { id } => {
                    self.handle_unsubscribe(client_id, id);
                    None
                }
                Message::Set { address, value } => {
                    self.state.set(address, value);
                    None // Broadcast handled separately via get_broadcast_targets
                }
                Message::Ping => Some(self.prepare_pong()),
                _ => None,
            }
        }

        /// Get list of clients that should receive a broadcast for an address
        ///
        /// Call this after processing a SET to get which clients need the update
        pub fn get_broadcast_targets(&self, address: &str, sender_id: u8) -> BroadcastList {
            let mut result = BroadcastList::empty();

            for (i, session) in self.sessions.iter().enumerate() {
                // Don't send back to sender, only to other active sessions with matching subs
                if session.active && i as u8 != sender_id && session.has_match(address) {
                    result.clients[i] = true;
                    result.count += 1;
                }
            }

            result
        }

        /// Prepare a SET frame for broadcasting to subscribers
        ///
        /// Returns the frame bytes to send to each matching client
        pub fn prepare_broadcast(&mut self, address: &str, value: Value) -> &[u8] {
            let n = encode_set_frame(&mut self.tx_buf, address, &value);
            &self.tx_buf[..n]
        }

        fn handle_subscribe(&mut self, client_id: u8, id: u32, pattern: &str) {
            if let Some(session) = self.sessions.get_mut(client_id as usize) {
                if session.active {
                    session.subscribe(id, pattern);
                }
            }
        }

        fn handle_unsubscribe(&mut self, client_id: u8, id: u32) {
            if let Some(session) = self.sessions.get_mut(client_id as usize) {
                if session.active {
                    session.unsubscribe(id);
                }
            }
        }

        fn create_session(&mut self, client_id: u8) {
            if (client_id as usize) < MAX_CLIENTS {
                self.sessions[client_id as usize] = Session {
                    active: true,
                    id: client_id,
                    subscriptions: [const { Subscription::empty() }; MAX_SUBS_PER_CLIENT],
                    sub_count: 0,
                };
                self.session_count += 1;
            }
        }

        /// Remove a client session
        pub fn disconnect(&mut self, client_id: u8) {
            if let Some(session) = self.sessions.get_mut(client_id as usize) {
                if session.active {
                    session.active = false;
                    session.sub_count = 0;
                    self.session_count = self.session_count.saturating_sub(1);
                }
            }
        }

        fn prepare_welcome(&mut self, _client_id: u8) -> &[u8] {
            let payload_start = HEADER_SIZE;
            let mut offset = payload_start;

            self.tx_buf[offset] = msg::WELCOME;
            offset += 1;

            self.tx_buf[offset] = VERSION;
            offset += 1;

            self.tx_buf[offset] = 0xF8; // param|event|stream|gesture|timeline
            offset += 1;

            self.tx_buf[offset..offset + 8].copy_from_slice(&0u64.to_be_bytes());
            offset += 8;

            offset += encode_string(&mut self.tx_buf[offset..], "embedded");
            offset += encode_string(&mut self.tx_buf[offset..], "MiniRouter");

            let payload_len = offset - payload_start;
            encode_header(&mut self.tx_buf, 0, payload_len);

            &self.tx_buf[..offset]
        }

        fn prepare_pong(&mut self) -> &[u8] {
            let n = encode_pong_frame(&mut self.tx_buf);
            &self.tx_buf[..n]
        }

        pub fn get(&self, address: &str) -> Option<Value> {
            self.state.get(address)
        }

        pub fn set(&mut self, address: &str, value: Value) {
            self.state.set(address, value);
        }

        /// Get number of active sessions
        pub fn session_count(&self) -> u8 {
            self.session_count
        }

        /// Get mutable access to a session (for testing/setup)
        pub fn session_mut(&mut self, client_id: u8) -> Option<&mut Session> {
            self.sessions.get_mut(client_id as usize)
        }
    }

    impl Default for MiniRouter {
        fn default() -> Self {
            Self::new()
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_value() {
        let mut buf = [0u8; 16];

        // Float
        let n = encode_value(&mut buf, &Value::Float(3.14));
        assert_eq!(n, 9);
        let (v, consumed) = decode_value(&buf).unwrap();
        assert_eq!(consumed, 9);
        assert!((v.as_float().unwrap() - 3.14).abs() < 0.001);

        // Int
        let n = encode_value(&mut buf, &Value::Int(-42));
        let (v, _) = decode_value(&buf).unwrap();
        assert_eq!(v.as_int(), Some(-42));
    }

    #[test]
    fn test_encode_decode_set() {
        let mut buf = [0u8; 64];
        let n = encode_set_frame(&mut buf, "/test/value", &Value::Float(1.5));
        assert!(n > HEADER_SIZE);

        let (_, payload_len) = decode_header(&buf).unwrap();
        let payload = &buf[HEADER_SIZE..HEADER_SIZE + payload_len];
        let msg = decode_message(payload).unwrap();

        match msg {
            Message::Set { address, value } => {
                assert_eq!(address, "/test/value");
                assert!((value.as_float().unwrap() - 1.5).abs() < 0.001);
            }
            _ => panic!("Expected Set message"),
        }
    }

    #[test]
    fn test_client_flow() {
        let mut client = Client::new();
        assert_eq!(client.state, ClientState::Disconnected);

        // Prepare hello
        let hello = client.prepare_hello("ESP32");
        assert!(hello.len() > HEADER_SIZE);

        // Simulate welcome response (binary format: type + version + features + time + session + name)
        let mut welcome_buf = [0u8; 64];
        let payload_start = HEADER_SIZE;
        let mut offset = payload_start;

        // Message type
        welcome_buf[offset] = msg::WELCOME;
        offset += 1;

        // Version
        welcome_buf[offset] = VERSION;
        offset += 1;

        // Features flags
        welcome_buf[offset] = 0xF8;
        offset += 1;

        // Server time (u64)
        welcome_buf[offset..offset + 8].copy_from_slice(&0u64.to_be_bytes());
        offset += 8;

        // Session ID
        offset += encode_string(&mut welcome_buf[offset..], "session123");

        // Server name
        offset += encode_string(&mut welcome_buf[offset..], "TestRouter");

        encode_header(&mut welcome_buf, 0, offset - payload_start);

        client.process(&welcome_buf[..offset]);
        assert_eq!(client.state, ClientState::Connected);
    }

    #[test]
    fn test_state_cache() {
        let mut cache = StateCache::new();

        cache.set("/sensor/temp", Value::Float(25.5));
        cache.set("/sensor/humidity", Value::Float(60.0));

        assert_eq!(cache.get("/sensor/temp").unwrap().as_float(), Some(25.5));
        assert_eq!(
            cache.get("/sensor/humidity").unwrap().as_float(),
            Some(60.0)
        );
        assert!(cache.get("/unknown").is_none());
    }

    #[test]
    fn test_memory_size() {
        let client_size = core::mem::size_of::<Client>();
        let cache_size = core::mem::size_of::<StateCache>();

        // Client should be under 4KB
        assert!(
            client_size < 4096,
            "Client too large: {} bytes",
            client_size
        );

        // Total memory budget check
        let total = client_size + 1024; // + some working memory
        assert!(total < 8192, "Total too large: {} bytes", total);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_mini_router() {
        use server::MiniRouter;

        let mut router = MiniRouter::new();
        router.set("/light/brightness", Value::Float(0.8));

        assert_eq!(
            router.get("/light/brightness").unwrap().as_float(),
            Some(0.8)
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_mini_router_subscriptions() {
        use server::{MiniRouter, Session, Subscription};

        // Test subscription pattern matching
        let mut sub = Subscription::empty();
        sub.active = true;
        sub.pattern_len = "/light/*".len();
        sub.pattern[..sub.pattern_len].copy_from_slice(b"/light/*");

        assert!(sub.matches("/light/brightness"));
        assert!(sub.matches("/light/color"));
        assert!(!sub.matches("/audio/volume"));
        assert!(!sub.matches("/light/zone/1"));

        // Test ** wildcard
        sub.pattern_len = "/light/**".len();
        sub.pattern[..sub.pattern_len].copy_from_slice(b"/light/**");
        assert!(sub.matches("/light/brightness"));
        assert!(sub.matches("/light/zone/1/brightness"));
        assert!(!sub.matches("/audio/volume"));

        // Test session subscriptions
        let mut session = Session::new();
        session.active = true;
        assert!(session.subscribe(1, "/light/*"));
        assert!(session.subscribe(2, "/audio/**"));
        assert_eq!(session.sub_count, 2);

        assert!(session.has_match("/light/brightness"));
        assert!(session.has_match("/audio/master/volume"));
        assert!(!session.has_match("/midi/cc/1"));

        // Unsubscribe
        assert!(session.unsubscribe(1));
        assert_eq!(session.sub_count, 1);
        assert!(!session.has_match("/light/brightness"));
        assert!(session.has_match("/audio/master/volume"));
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_mini_router_broadcast() {
        use server::MiniRouter;

        let mut router = MiniRouter::new();

        // Simulate two clients connecting
        // Client 0 subscribes to /light/**
        // Client 1 subscribes to /audio/**

        // Create sessions manually (normally done via HELLO message)
        {
            let session = router.session_mut(0).unwrap();
            session.active = true;
            session.id = 0;
            session.subscribe(1, "/light/**");
        }

        {
            let session = router.session_mut(1).unwrap();
            session.active = true;
            session.id = 1;
            session.subscribe(1, "/audio/**");
        }

        // Client 2 sends a SET to /light/brightness
        router.set("/light/brightness", Value::Float(0.75));

        // Get broadcast targets (excluding sender 2)
        let targets = router.get_broadcast_targets("/light/brightness", 2);
        assert_eq!(targets.count, 1);
        assert!(targets.clients[0]); // Client 0 should receive
        assert!(!targets.clients[1]); // Client 1 should NOT receive

        // Test audio broadcast
        let targets = router.get_broadcast_targets("/audio/volume", 2);
        assert_eq!(targets.count, 1);
        assert!(!targets.clients[0]); // Client 0 should NOT receive
        assert!(targets.clients[1]); // Client 1 should receive

        // Test that sender doesn't receive their own broadcast
        let targets = router.get_broadcast_targets("/light/brightness", 0);
        assert_eq!(targets.count, 0); // Client 0 sent it, so no one else matches
    }
}
