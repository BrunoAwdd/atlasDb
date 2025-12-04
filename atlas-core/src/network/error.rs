use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to send message: {0}")]
    Send(String),

    #[error("Failed to receive message: {0}")]
    Receive(String),

    #[error("Message handler not configured")]
    HandlerNotSet,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Invalid message error")]
    InvalidMessage,

    #[error("Unknown error")]
    Unknown,
}
