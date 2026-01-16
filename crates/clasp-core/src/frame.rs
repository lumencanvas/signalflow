//! Binary frame encoding/decoding
//!
//! SignalFlow frame format:
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Byte 0:     Magic (0x53 = 'S')                                  │
//! │ Byte 1:     Flags                                               │
//! │             [7:6] QoS (00=fire, 01=confirm, 10=commit, 11=rsv)  │
//! │             [5]   Timestamp present                             │
//! │             [4]   Encrypted                                     │
//! │             [3]   Compressed                                    │
//! │             [2:0] Reserved                                      │
//! │ Byte 2-3:   Payload Length (uint16 big-endian, max 65535)       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ [If timestamp flag] Bytes 4-11: Timestamp (uint64 µs)           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Payload (MessagePack encoded)                                   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use crate::{Error, QoS, Result, MAGIC_BYTE};
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// Frame header size without timestamp
pub const HEADER_SIZE: usize = 4;

/// Frame header size with timestamp
pub const HEADER_SIZE_WITH_TS: usize = 12;

/// Maximum payload size
pub const MAX_PAYLOAD_SIZE: usize = 65535;

/// Frame flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags {
    pub qos: QoS,
    pub has_timestamp: bool,
    pub encrypted: bool,
    pub compressed: bool,
}

impl FrameFlags {
    pub fn to_byte(&self) -> u8 {
        let mut flags = 0u8;
        flags |= (self.qos as u8) << 6;
        if self.has_timestamp {
            flags |= 0x20;
        }
        if self.encrypted {
            flags |= 0x10;
        }
        if self.compressed {
            flags |= 0x08;
        }
        flags
    }

    pub fn from_byte(byte: u8) -> Self {
        Self {
            qos: QoS::from_u8((byte >> 6) & 0x03).unwrap_or(QoS::Fire),
            has_timestamp: (byte & 0x20) != 0,
            encrypted: (byte & 0x10) != 0,
            compressed: (byte & 0x08) != 0,
        }
    }
}

/// A SignalFlow frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub flags: FrameFlags,
    pub timestamp: Option<u64>,
    pub payload: Bytes,
}

impl Frame {
    /// Create a new frame with payload
    pub fn new(payload: impl Into<Bytes>) -> Self {
        Self {
            flags: FrameFlags::default(),
            timestamp: None,
            payload: payload.into(),
        }
    }

    /// Create a frame with QoS
    pub fn with_qos(mut self, qos: QoS) -> Self {
        self.flags.qos = qos;
        self
    }

    /// Create a frame with timestamp
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self.flags.has_timestamp = true;
        self
    }

    /// Create a frame with encryption flag
    pub fn with_encrypted(mut self, encrypted: bool) -> Self {
        self.flags.encrypted = encrypted;
        self
    }

    /// Create a frame with compression flag
    pub fn with_compressed(mut self, compressed: bool) -> Self {
        self.flags.compressed = compressed;
        self
    }

    /// Calculate the total frame size
    pub fn size(&self) -> usize {
        let header = if self.flags.has_timestamp {
            HEADER_SIZE_WITH_TS
        } else {
            HEADER_SIZE
        };
        header + self.payload.len()
    }

    /// Encode frame to bytes
    pub fn encode(&self) -> Result<Bytes> {
        if self.payload.len() > MAX_PAYLOAD_SIZE {
            return Err(Error::PayloadTooLarge(self.payload.len()));
        }

        let mut buf = BytesMut::with_capacity(self.size());

        // Magic byte
        buf.put_u8(MAGIC_BYTE);

        // Flags
        buf.put_u8(self.flags.to_byte());

        // Payload length
        buf.put_u16(self.payload.len() as u16);

        // Timestamp (if present)
        if let Some(ts) = self.timestamp {
            buf.put_u64(ts);
        }

        // Payload
        buf.extend_from_slice(&self.payload);

        Ok(buf.freeze())
    }

    /// Decode frame from bytes
    pub fn decode(mut buf: impl Buf) -> Result<Self> {
        if buf.remaining() < HEADER_SIZE {
            return Err(Error::BufferTooSmall {
                needed: HEADER_SIZE,
                have: buf.remaining(),
            });
        }

        // Magic byte
        let magic = buf.get_u8();
        if magic != MAGIC_BYTE {
            return Err(Error::InvalidMagic(magic));
        }

        // Flags
        let flags = FrameFlags::from_byte(buf.get_u8());

        // Payload length
        let payload_len = buf.get_u16() as usize;

        // Calculate required size
        let header_size = if flags.has_timestamp {
            HEADER_SIZE_WITH_TS
        } else {
            HEADER_SIZE
        };
        let total_remaining = if flags.has_timestamp { 8 } else { 0 } + payload_len;

        if buf.remaining() < total_remaining {
            return Err(Error::BufferTooSmall {
                needed: header_size + payload_len,
                have: HEADER_SIZE + buf.remaining(),
            });
        }

        // Timestamp
        let timestamp = if flags.has_timestamp {
            Some(buf.get_u64())
        } else {
            None
        };

        // Payload
        let payload = buf.copy_to_bytes(payload_len);

        Ok(Self {
            flags,
            timestamp,
            payload,
        })
    }

    /// Check if buffer contains a complete frame
    pub fn check_complete(buf: &[u8]) -> Option<usize> {
        if buf.len() < HEADER_SIZE {
            return None;
        }

        if buf[0] != MAGIC_BYTE {
            return None;
        }

        let flags = FrameFlags::from_byte(buf[1]);
        let payload_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;

        let header_size = if flags.has_timestamp {
            HEADER_SIZE_WITH_TS
        } else {
            HEADER_SIZE
        };

        let total_size = header_size + payload_len;

        if buf.len() >= total_size {
            Some(total_size)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_encode_decode() {
        let payload = b"hello world";
        let frame = Frame::new(payload.as_slice())
            .with_qos(QoS::Confirm)
            .with_timestamp(1234567890);

        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded[..]).unwrap();

        assert_eq!(decoded.flags.qos, QoS::Confirm);
        assert_eq!(decoded.timestamp, Some(1234567890));
        assert_eq!(decoded.payload.as_ref(), payload);
    }

    #[test]
    fn test_flags_roundtrip() {
        let flags = FrameFlags {
            qos: QoS::Commit,
            has_timestamp: true,
            encrypted: true,
            compressed: false,
        };

        let byte = flags.to_byte();
        let decoded = FrameFlags::from_byte(byte);

        assert_eq!(decoded.qos, QoS::Commit);
        assert!(decoded.has_timestamp);
        assert!(decoded.encrypted);
        assert!(!decoded.compressed);
    }

    #[test]
    fn test_check_complete() {
        let frame = Frame::new(b"test".as_slice());
        let encoded = frame.encode().unwrap();

        // Complete frame
        assert_eq!(Frame::check_complete(&encoded), Some(encoded.len()));

        // Incomplete header
        assert_eq!(Frame::check_complete(&encoded[..2]), None);

        // Incomplete payload
        assert_eq!(Frame::check_complete(&encoded[..5]), None);
    }
}
