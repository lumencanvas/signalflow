//! Error types for SignalFlow

use thiserror::Error;

/// Result type alias for SignalFlow operations
pub type Result<T> = std::result::Result<T, Error>;

/// SignalFlow error types
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid magic byte in frame header
    #[error("invalid magic byte: expected 0x53, got 0x{0:02x}")]
    InvalidMagic(u8),

    /// Frame payload too large
    #[error("payload too large: {0} bytes (max 65535)")]
    PayloadTooLarge(usize),

    /// Frame buffer too small
    #[error("buffer too small: need {needed} bytes, have {have}")]
    BufferTooSmall { needed: usize, have: usize },

    /// MessagePack encoding error
    #[error("encode error: {0}")]
    EncodeError(String),

    /// MessagePack decoding error
    #[error("decode error: {0}")]
    DecodeError(String),

    /// Invalid message type code
    #[error("unknown message type: 0x{0:02x}")]
    UnknownMessageType(u8),

    /// Invalid signal type
    #[error("unknown signal type: {0}")]
    UnknownSignalType(String),

    /// Invalid address format
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Address pattern compilation error
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),

    /// State conflict
    #[error("state conflict: revision {expected} expected, got {actual}")]
    RevisionConflict { expected: u64, actual: u64 },

    /// Lock held by another session
    #[error("lock held by {holder}")]
    LockHeld { holder: String },

    /// Permission denied
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Connection error
    #[error("connection error: {0}")]
    ConnectionError(String),

    /// Timeout
    #[error("operation timed out")]
    Timeout,

    /// Generic protocol error
    #[error("protocol error: {0}")]
    Protocol(String),
}

impl From<rmp_serde::encode::Error> for Error {
    fn from(e: rmp_serde::encode::Error) -> Self {
        Error::EncodeError(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(e: rmp_serde::decode::Error) -> Self {
        Error::DecodeError(e.to_string())
    }
}

/// Protocol error codes (for ERROR messages)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ErrorCode {
    // 100-199: Protocol errors
    InvalidFrame = 100,
    InvalidMessage = 101,
    UnsupportedVersion = 102,

    // 200-299: Address errors
    InvalidAddress = 200,
    AddressNotFound = 201,
    PatternError = 202,

    // 300-399: Permission errors
    Unauthorized = 300,
    Forbidden = 301,
    TokenExpired = 302,

    // 400-499: State errors
    RevisionConflict = 400,
    LockHeld = 401,
    InvalidValue = 402,

    // 500-599: Server errors
    InternalError = 500,
    ServiceUnavailable = 501,
    Timeout = 502,
}

impl ErrorCode {
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            100 => Some(ErrorCode::InvalidFrame),
            101 => Some(ErrorCode::InvalidMessage),
            102 => Some(ErrorCode::UnsupportedVersion),
            200 => Some(ErrorCode::InvalidAddress),
            201 => Some(ErrorCode::AddressNotFound),
            202 => Some(ErrorCode::PatternError),
            300 => Some(ErrorCode::Unauthorized),
            301 => Some(ErrorCode::Forbidden),
            302 => Some(ErrorCode::TokenExpired),
            400 => Some(ErrorCode::RevisionConflict),
            401 => Some(ErrorCode::LockHeld),
            402 => Some(ErrorCode::InvalidValue),
            500 => Some(ErrorCode::InternalError),
            501 => Some(ErrorCode::ServiceUnavailable),
            502 => Some(ErrorCode::Timeout),
            _ => None,
        }
    }
}
