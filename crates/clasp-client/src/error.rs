//! Client error types

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("not connected")]
    NotConnected,

    #[error("already connected")]
    AlreadyConnected,

    #[error("send failed: {0}")]
    SendFailed(String),

    #[error("timeout")]
    Timeout,

    #[error("protocol error: {0}")]
    Protocol(#[from] clasp_core::Error),

    #[error("transport error: {0}")]
    Transport(#[from] clasp_transport::TransportError),

    #[error("client error: {0}")]
    Other(String),
}
