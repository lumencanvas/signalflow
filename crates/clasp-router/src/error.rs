//! Router error types

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RouterError>;

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("routing error: {0}")]
    Routing(String),

    #[error("state error: {0}")]
    State(String),

    #[error("transport error: {0}")]
    Transport(#[from] clasp_transport::TransportError),

    #[error("protocol error: {0}")]
    Protocol(#[from] clasp_core::Error),

    #[error("router error: {0}")]
    Other(String),
}
