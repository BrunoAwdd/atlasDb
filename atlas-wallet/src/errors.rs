use thiserror::Error;

#[derive(Debug, Error)]
pub enum NimbleError {
    #[error("Address error: {0}")]
    Address(#[from] atlas_common::address::errors::AddressError),

    #[error("Identity error: {0}")]
    Identity(#[from] crate::identity::errors::IdentityError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] atlas_common::transactions::errors::TransactionError),
    
    #[error("General error: {0}")]
    General(String)
}

impl From<String> for NimbleError {
    fn from(message: String) -> Self {
        NimbleError::General(message)
    }
}

impl From<&str> for NimbleError {
    fn from(message: &str) -> Self {
        NimbleError::General(message.to_string())
    }
}