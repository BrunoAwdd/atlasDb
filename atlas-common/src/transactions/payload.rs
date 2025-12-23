use crate::transactions::TransferRequest;

use super::errors::TransactionError;

/// Represents the core data of a transfer transaction.
///
/// A `TransferPayload` defines the essential elements of a value transfer: the sender ID,
/// the recipient ID, and the amount being transferred. This structure is lightweight
/// and used primarily for signing, verification, or serialization.
///
/// It is intentionally detached from cryptographic or contextual metadata to ensure
/// deterministic behavior when converting to/from strings or hashes.
///
/// # Fields
///
/// - `from_id`: The unique address or identifier of the sender.
/// - `to_id`: The unique address or identifier of the recipient.
/// - `amount`: The amount of value to be transferred.
///
/// # Example
///
/// ```rust
/// let from = Profile { address: "user1".into() };
/// let to = Profile { address: "user2".into() };
/// let payload = TransferPayload::new(&from, &to, 100);
/// assert_eq!(payload.amount, 100);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TransferPayload {
    pub from_id: String,
    pub to_id: String,
    pub amount: u64,
    pub timestamp: u64,
    pub nonce: u64,
}

impl TransferPayload {
    /// Creates a new `TransferPayload` from two profiles and an amount.
    ///
    /// This method initializes a new payload with the sender and recipient profile addresses,
    /// and the amount of the transfer.
    ///
    /// # Arguments
    ///
    /// * `from` - The sender's profile.
    /// * `to` - The recipient's profile.
    /// * `amount` - The amount to be transferred.
    ///
    /// # Returns
    /// A `TransferPayload` instance with the given values.
    pub fn new(from: &str, to: String, amount: u64, nonce: u64) -> Self {
        Self {
            from_id: from.to_string(),
            to_id: to,
            amount,
            timestamp: crate::utils::time::current_time(),
            nonce,
        }
    }

    pub fn new_from_request(request: &TransferRequest) -> Self {
        Self {
            from_id: request.from.to_string(),
            to_id: request.to.to_string(),
            amount: request.amount,
            timestamp: request.timestamp,
            nonce: request.nonce,
        }
    }

    /// Converts the payload into a canonical string format.
    ///
    /// The format used is: `"transfer:{from_id}->{to_id}:${amount}"`.
    /// This is typically used for signing, hashing, or transmission.
    ///
    /// # Returns
    /// A string representing the payload.
    ///
    /// # Example
    ///
    /// ```rust
    /// let payload = TransferPayload {
    ///     from_id: "alice".into(),
    ///     to_id: "bob".into(),
    ///     amount: 42,
    /// };
    /// assert_eq!(payload.to_string(), "transfer:alice->bob:$42@1725000000#42");
    /// ```
    pub fn to_string(&self) -> String {
        format!("transfer:{}->{}:${}@{}#{}", self.from_id, self.to_id, self.amount, self.timestamp, self.nonce)
    }

    /// Parses a payload from a string in canonical format.
    ///
    /// This performs strict validation on the format:
    /// - Must start with `"transfer:"`
    /// - Must contain the separator `"->"` and `":$"`
    /// - Sender and recipient IDs must not be empty
    /// - Amount must be a valid `u64` integer
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse.
    ///
    /// # Errors
    ///
    /// Returns a `TransactionError` if the string is malformed or invalid.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `TransferPayload`, or an error if parsing fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = "transfer:alice->bob:$42@1725000000#42";
    /// let payload = TransferPayload::from_string(s).unwrap();
    /// assert_eq!(payload.from_id, "alice");
    /// ```

    pub fn from_string(s: &str) -> Result<Self, TransactionError> {
        let s = s.strip_prefix("transfer:")
            .ok_or_else(|| TransactionError::InvalidPayload("Transfer prefix not found".into()))?;
    
        let (from_part, rest) = s.split_once("->")
            .ok_or_else(|| TransactionError::InvalidPayload("Missing '->'".into()))?;
    
        let (to_part, value_part) = rest.split_once(":$")
            .ok_or_else(|| TransactionError::InvalidPayload("Missing ':$'".into()))?;
    
        // Agora lidamos com `value_part`: "42@1725000000#42"
        let (amount_str, rest) = value_part.split_once('@')
            .ok_or_else(|| TransactionError::InvalidPayload("Missing '@timestamp'".into()))?;
    
        let (timestamp_str, nonce_str) = rest.split_once('#')
            .ok_or_else(|| TransactionError::InvalidPayload("Missing '#nonce'".into()))?;
    
        let amount = amount_str.parse::<u64>()
            .map_err(|_| TransactionError::InvalidPayload("Invalid amount".into()))?;
    
        let timestamp = timestamp_str.parse::<u64>()
            .map_err(|_| TransactionError::InvalidPayload("Invalid timestamp".into()))?;
    
        let nonce = nonce_str.parse::<u64>()
            .map_err(|_| TransactionError::InvalidPayload("Invalid nonce".into()))?;
    
        if from_part.is_empty() || to_part.is_empty() {
            return Err(TransactionError::InvalidPayload("from_id or to_id cannot be empty".into()));
        }
    
        Ok(Self {
            from_id: from_part.to_string(),
            to_id: to_part.to_string(),
            amount,
            timestamp,
            nonce,
        })
    }
    
    

    /// Checks whether the payload is valid.
    ///
    /// A valid payload must have:
    /// - Non-empty `from_id` and `to_id`
    /// - A positive `amount`
    ///
    /// # Returns
    ///
    /// `true` if the payload is valid; `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// let payload = TransferPayload {
    ///     from_id: "alice".into(),
    ///     to_id: "bob".into(),
    ///     amount: 0,
    /// };
    /// assert!(!payload.is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        !self.from_id.is_empty() && !self.to_id.is_empty() && self.amount > 0
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::errors::TransactionError;

    #[test]
    fn test_new_payload_creates_correct_fields() {
        let payload = TransferPayload::new("alice", "bob".to_string(), 100, 1);
        assert_eq!(payload.from_id, "alice");
        assert_eq!(payload.to_id, "bob");
        assert_eq!(payload.amount, 100);
        assert_eq!(payload.nonce, 1);
    }

    #[test]
    fn test_to_string_format() {
        let payload = TransferPayload::new("alice", "bob".to_string(), 42, 2);
        let s = payload.to_string();
        assert_eq!(s, "transfer:alice->bob:$42#2");
    }

    #[test]
    fn test_from_string_parses_correctly() {
        let s = "transfer:alice->bob:$500@1725000000#42";
        let payload = TransferPayload::from_string(s).unwrap();
        assert_eq!(payload.from_id, "alice");
        assert_eq!(payload.to_id, "bob");
        assert_eq!(payload.amount, 500);
        assert_eq!(payload.nonce, 42);
        assert_eq!(payload.timestamp, 1725000000);
    }

    #[test]
    fn test_from_string_missing_prefix() {
        let s = "alice->bob:$100";
        let result = TransferPayload::from_string(s);
        assert!(matches!(result, Err(TransactionError::InvalidPayload(_))));
    }

    #[test]
    fn test_from_string_invalid_separator() {
        let s = "transfer:alice=bob:$100";
        let result = TransferPayload::from_string(s);
        assert!(matches!(result, Err(TransactionError::InvalidPayload(_))));
    }

    #[test]
    fn test_from_string_invalid_amount_format() {
        let s = "transfer:alice->bob:$notanumber";
        let result = TransferPayload::from_string(s);
        assert!(matches!(result, Err(TransactionError::InvalidPayload(_))));
    }

    #[test]
    fn test_from_string_empty_fields() {
        let s = "transfer:->:$100";
        let result = TransferPayload::from_string(s);
        assert!(matches!(result, Err(TransactionError::InvalidPayload(_))));
    }

    #[test]
    fn test_is_valid_true() {
        let payload = TransferPayload::new("alice", "bob".to_string(), 1, 3);
        assert!(payload.is_valid());
    }

    #[test]
    fn test_is_valid_false_on_empty_fields() {
        let payload = TransferPayload::new("", "bob".to_string(), 1, 2);
        assert!(!payload.is_valid());

        let payload = TransferPayload::new("alice", "".to_string(), 1, 1);
        assert!(!payload.is_valid());
    }

    #[test]
    fn test_is_valid_false_on_zero_amount() {
        let payload = TransferPayload::new("alice", "bob".to_string(), 0, 0);
        assert!(!payload.is_valid());
    }
}
