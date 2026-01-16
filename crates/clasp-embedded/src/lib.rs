//! SignalFlow Embedded
//!
//! Minimal no_std implementation for embedded devices.
//! This crate provides the "Lite" profile for constrained devices.
//!
//! # Features
//! - Fixed 2-byte addresses (numeric IDs)
//! - Minimal message set
//! - No compression/encryption
//! - UDP only
//!
//! # Memory Requirements
//! - < 4KB RAM
//! - < 16KB Flash

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

// TODO: Implement lite protocol
// - Numeric addresses
// - Fixed-size messages
// - Simple state machine

/// Lite message types
#[repr(u8)]
pub enum LiteMessageType {
    Hello = 0x01,
    Welcome = 0x02,
    Set = 0x21,
    Publish = 0x20,
    Ping = 0x41,
    Pong = 0x42,
}

/// Lite frame header (fixed 4 bytes)
#[repr(C, packed)]
pub struct LiteHeader {
    pub magic: u8,      // 0x53
    pub msg_type: u8,   // LiteMessageType
    pub address: u16,   // Numeric address
}

/// Lite SET message
#[repr(C, packed)]
pub struct LiteSetMessage {
    pub header: LiteHeader,
    pub value: i32,     // Fixed 32-bit value
}

/// Encode a lite SET message
pub fn encode_lite_set(address: u16, value: i32, buf: &mut [u8]) -> usize {
    if buf.len() < 8 {
        return 0;
    }

    buf[0] = 0x53; // Magic
    buf[1] = LiteMessageType::Set as u8;
    buf[2] = (address >> 8) as u8;
    buf[3] = address as u8;
    buf[4] = (value >> 24) as u8;
    buf[5] = (value >> 16) as u8;
    buf[6] = (value >> 8) as u8;
    buf[7] = value as u8;

    8
}

/// Decode a lite message header
pub fn decode_lite_header(buf: &[u8]) -> Option<(LiteMessageType, u16)> {
    if buf.len() < 4 || buf[0] != 0x53 {
        return None;
    }

    let msg_type = match buf[1] {
        0x01 => LiteMessageType::Hello,
        0x02 => LiteMessageType::Welcome,
        0x21 => LiteMessageType::Set,
        0x20 => LiteMessageType::Publish,
        0x41 => LiteMessageType::Ping,
        0x42 => LiteMessageType::Pong,
        _ => return None,
    };

    let address = ((buf[2] as u16) << 8) | (buf[3] as u16);

    Some((msg_type, address))
}
