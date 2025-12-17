use thiserror::Error;

/// Represents errors that can occur during transaction processing,
/// including permission checks, signature validation, and payload integrity.
#[derive(Debug, Error)]
pub enum TransactionError {
    /// The provided signature (sig2) is invalid or does not match the payload.
    ///
    /// This error typically occurs when:
    /// - The signature length is incorrect.
    /// - The signature does not match the expected HMAC result.
    /// - The password-derived secret used for signing is incorrect.
    #[error("Invalid password signature: {0}")]
    InvalidPasswordSignature(String),

    /// The sender does not have permission to perform the transfer operation.
    ///
    /// This error occurs when the `from` profile is missing the `"transfer"` permission
    /// in its permission set.
    #[error("Permission denied: {0}")]
    InvalidPermission(String),

    /// The transfer payload is malformed or does not meet required constraints.
    ///
    /// This can happen if:
    /// - `from_id` or `to_id` are empty.
    /// - The amount is zero or negative.
    /// - The payload string format is invalid during parsing.
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
}
