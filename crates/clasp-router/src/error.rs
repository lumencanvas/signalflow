//! Router error types

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RouterError>;

#[cfg(feature = "mqtt-server")]
impl From<mqttbytes::Error> for RouterError {
    fn from(e: mqttbytes::Error) -> Self {
        RouterError::Mqtt(e)
    }
}

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

    #[error("configuration error: {0}")]
    Config(String),

    #[error("transport error: {0}")]
    Transport(#[from] clasp_transport::TransportError),

    #[error("core protocol error: {0}")]
    Core(#[from] clasp_core::Error),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("router error: {0}")]
    Other(String),

    #[cfg(feature = "mqtt-server")]
    #[error("MQTT protocol error: {0:?}")]
    Mqtt(mqttbytes::Error),
}
