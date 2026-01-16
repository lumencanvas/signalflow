//! Bridge error types

use thiserror::Error;

pub type Result<T> = std::result::Result<T, BridgeError>;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("mapping error: {0}")]
    Mapping(String),

    #[error("send error: {0}")]
    Send(String),

    #[error("receive error: {0}")]
    Receive(String),

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bridge error: {0}")]
    Other(String),
}
