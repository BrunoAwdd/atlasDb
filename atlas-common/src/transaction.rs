use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u128,
    pub asset: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

pub fn signing_bytes(tx: &Transaction) -> Vec<u8> {
    bincode::serialize(tx).unwrap()
}

impl SignedTransaction {
    /// Performs stateless validation checks:
    /// 1. Signature Verification.
    /// 2. Address Derivation (from == public_key).
    /// 3. Destination Address Format (Base58, 32 bytes).
    /// 4. Memo length limit (256).
    /// 5. Amount > 0.
    pub fn validate_stateless(&self) -> Result<(), String> {
        // 1. Check Amount
        if self.transaction.amount == 0 {
            return Err("Transaction amount must be greater than 0".to_string());
        }

        // 2. Check Memo Length
        if let Some(memo) = &self.transaction.memo {
            if memo.len() > 256 {
                return Err("Memo too long (max 256 bytes)".to_string());
            }
        }

        // 3. Verify Public Key Format (32 bytes)
        if self.public_key.len() != 32 {
            return Err("Invalid public key length (must be 32 bytes)".to_string());
        }

        // 4. Check 'From' Address Derivation
        // The 'from' address MUST be the Base58 encoding of the signer's public key.
        let expected_from = bs58::encode(&self.public_key).into_string();
        if self.transaction.from != expected_from {
            return Err(format!("Invalid 'from' address. Expected: {}, Got: {}", expected_from, self.transaction.from));
        }

        // 5. Check 'To' Address Format
        // Must be valid Base58 and decode to exactly 32 bytes.
        let to_bytes = bs58::decode(&self.transaction.to).into_vec()
            .map_err(|e| format!("Invalid 'to' address format: {}", e))?;
        if to_bytes.len() != 32 {
             return Err(format!("Invalid 'to' address length: {} (expected 32)", to_bytes.len()));
        }

        // 6. Verify Signature
        use ed25519_dalek::{Verifier, VerifyingKey, Signature};

        let verifying_key = VerifyingKey::from_bytes(self.public_key.as_slice().try_into().unwrap())
            .map_err(|_| "Invalid public key bytes".to_string())?;
        
        // We assume signature is 64 bytes
        if self.signature.len() != 64 {
             return Err("Invalid signature length".to_string());
        }
        let signature = Signature::from_slice(&self.signature)
             .map_err(|_| "Invalid signature format".to_string())?;
        
        let msg = signing_bytes(&self.transaction);
        verifying_key.verify(&msg, &signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        Ok(())
    }
}
