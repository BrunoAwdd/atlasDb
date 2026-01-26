use serde::{Deserialize, Serialize};

use super::{errors::TransactionError, payload::TransferPayload};

/// Represents a signed transfer request between two profiles.
///
/// The `TransferRequest` struct contains all necessary information to execute a value transfer,
/// including sender and recipient profiles, the transfer amount, the cryptographic signature,
/// and an optional memo for additional context or description.
///
/// # Fields
///
/// - `from`: The sender's profile.
/// - `to`: The recipient's profile.
/// - `amount`: The amount to be transferred.
/// - `sig2`: A digital signature of the transfer payload (typically created using a password-derived secret).
/// - `memo`: An optional message or note to attach to the transfer.
///
/// # Example
///
/// ```rust
/// use nimble_protocol::core::transactions::request::TransferRequest;
/// use nimble_protocol::core::identity::profile::Profile;
/// 
/// let from = Profile { address: "addr1".into() };
/// let to = Profile { address: "addr2".into() };
/// let sig2 = vec![0, 1, 2];  // Example signature
///
/// let request = TransferRequest {
///     from: &from,
///     to: &to,
///     amount: 100,
///     sig2,
///     memo: Some("For services rendered".into()),
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    #[serde(with = "hex::serde")]
    pub sig2: [u8; 64],
    pub memo: Option<String>,
    pub timestamp: u64,
    pub nonce: u64,
}



impl TransferRequest {
    /// Converts the request into a [`TransferPayload`].
    ///
    /// This function extracts the sender, recipient, and amount fields and
    /// creates a lightweight payload representation suitable for serialization
    /// or signing.
    ///
    /// # Returns
    /// A `TransferPayload` containing the request’s core information.
    pub fn to_payload(&self) -> TransferPayload {
        TransferPayload::new_from_request(self)
    }

    pub fn from_json(json_str: &str) -> Result<Self, String> {
        serde_json::from_str(json_str).map_err(|e| e.to_string())
    }

    /// Converte TransferRequest para uma string JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    /// Returns the string representation of the payload.
    ///
    /// Useful for hashing, signing, or logging purposes.
    ///
    /// # Returns
    /// A string in the format: `"transfer:{from_id}->{to_id}:${amount}"`
    ///
    /// # Example
    /// ```rust
    /// use nimble_protocol::core::transactions::request::TransferRequest;
    /// 
    /// let request = TransferRequest::new(&from, &to, 50);
    /// let s = request.payload_string();
    /// assert!(s.starts_with("transfer:"));
    /// ```
    pub fn payload_string(&self) -> String {
        self.to_payload().to_string()
    }

    /// Reconstructs a `TransferRequest` from a payload and associated metadata.
    ///
    /// This is typically used when verifying a payload and attaching its signature
    /// and memo. It validates that the given profiles match the payload content.
    ///
    /// # Arguments
    ///
    /// * `payload` - The original `TransferPayload`.
    /// * `from` - The profile of the sender.
    /// * `to` - The profile of the recipient.
    /// * `sig2` - The signature associated with the payload.
    /// * `memo` - An optional memo to attach to the request.
    ///
    /// # Returns
    /// `Some(TransferRequest)` if `from.address` and `to.address` match the payload;
    /// otherwise, returns `None`.
    pub fn from_payload<'b>(
        payload: &TransferPayload,
        from: &'b str,
        to: &'b str,
        sig2: [u8; 64],
        memo: Option<String>,
        nonce: u64,
    ) -> Option<TransferRequest> {
        if from != payload.from_id || to != payload.to_id {
            return None;
        }

        Some(Self {
            from: from.to_string(),
            to: to.to_string(),
            amount: payload.amount,
            sig2,
            memo,
            timestamp: crate::utils::time::current_time(),
            nonce,
        })
    }

    /// Builds and signs a `TransferRequest` from the given information.
    ///
    /// This function validates the input, generates a deterministic secret using
    /// the provided password, signs the payload, and returns a complete request
    /// ready for submission or storage.
    ///
    /// # Arguments
    ///
    /// * `from` - Sender's profile.
    /// * `to` - Recipient's profile.
    /// * `amount` - The amount to transfer (must be greater than zero).
    /// * `password` - The sender’s password used to derive the signing key.
    /// * `memo` - Optional memo attached to the transfer.
    ///
    /// # Errors
    ///
    /// Returns a [`TransactionError`] if:
    /// - Any address is empty.
    /// - The amount is zero.
    /// - Password key derivation fails.
    /// - The payload signing fails.
    ///
    /// # Returns
    ///
    /// A fully signed `TransferRequest` ready for processing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nimble_protocol::core::transactions::request::build_signed_request;
    /// 
    /// let req = build_signed_request(&from, &to, 50, "hunter2", None)?;
    /// assert_eq!(req.amount, 50);
    /// ```
    pub fn build_signed_request(
        from: String,
        to: String,
        amount: u64,
        sig2: [u8; 64],
        memo: Option<String>,
        timestamp: u64,
        nonce: u64
    ) -> Result<TransferRequest, TransactionError> {
        let amt = amount.clone();
        if from.is_empty() || to.is_empty() {
            return Err(
                TransactionError::InvalidPayload("Sender or receiver address is empty".into()).into()
            );
        }

        if amt == 0 {
            return Err(
                TransactionError::InvalidPayload("Amount cannot be zero".into()).into()
            );
        }

        Ok(TransferRequest {
            from,
            to,
            amount: amt,
            sig2,
            memo,
            timestamp,
            nonce,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};
    use super::*;
    
    fn dummy_sig() -> [u8; 64] {
        [0u8; 64]
    }

    #[test]
    fn test_to_payload() {
        let req = TransferRequest {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            sig2: dummy_sig(),
            memo: None,
            timestamp: 0,
            nonce: 0,
        };

        let payload = req.to_payload();
        assert_eq!(payload.from_id, "alice");
        assert_eq!(payload.to_id, "bob");
        assert_eq!(payload.amount, 100);
    }

    #[test]
    fn test_payload_string_format() {
        let req = TransferRequest {
            from: "carol".to_string(),
            to: "dave".to_string(),
            amount: 42,
            sig2: dummy_sig(),
            memo: None,
            timestamp: 0,
            nonce: 0,
        };

        let s = req.payload_string();
        assert_eq!(s, "transfer:carol->dave:$42");
    }

    #[test]
    fn test_from_payload_success() {
        let payload = TransferPayload::new("eve", "frank".to_string(), 77, 0);
        let sig = dummy_sig();
        let memo = Some("payment".to_string());

        let request = TransferRequest::from_payload(
            &payload, "eve", 
            "frank", 
            sig.clone(), 
            memo.clone(),
            0,
        )
            .expect("Should create TransferRequest");

        assert_eq!(request.from, "eve");
        assert_eq!(request.to, "frank");
        assert_eq!(request.amount, 77);
        assert_eq!(request.sig2, sig);
        assert_eq!(request.memo, memo);
    }

    #[test]
    fn test_from_payload_mismatch_should_fail() {
        let payload = TransferPayload::new("eve", "frank".to_string(), 77, 2);
        let sig = dummy_sig();

        let result = TransferRequest::from_payload(&payload, "eve", "someone_else", sig, None, 0);

        assert!(result.is_none(), "Mismatched payload should return None");
    }

    #[test]
    fn test_build_signed_request_success() {
        let req = TransferRequest::build_signed_request(
            "alice".to_string(),
            "bob".to_string(),
            123,
            dummy_sig(),
            Some("gift".to_string()),
            crate::utils::time::current_time(),
            0
        ).expect("Should build request");

        assert_eq!(req.amount, 123);
        assert_eq!(req.memo.unwrap(), "gift");
    }

    #[test]
    fn test_build_signed_request_empty_from_should_fail() {
        let result = TransferRequest::build_signed_request(
            "".to_string(),
            "bob".to_string(),
            50,
            dummy_sig(),
            None,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_build_signed_request_empty_to_should_fail() {
        let result = TransferRequest::build_signed_request(
            "alice".to_string(),
            "".to_string(),
            50,
            dummy_sig(),
            None,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            0
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_build_signed_request_zero_amount_should_fail() {
        let result = TransferRequest::build_signed_request(
            "alice".to_string(),
            "bob".to_string(),
            0,
            dummy_sig(),
            None,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            0
        );
        assert!(result.is_err());
    }
}

