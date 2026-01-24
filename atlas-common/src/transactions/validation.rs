use crate::transactions::types::{SignedTransaction, signing_bytes};
use ed25519_dalek::{Verifier, VerifyingKey, Signature};

pub struct TransactionValidator;

impl TransactionValidator {
    /// Performs stateless validation checks:
    /// 1. Signature Verification.
    /// 2. Address Derivation (from == public_key).
    /// 3. Destination Address Format (Base58, 32 bytes).
    /// 4. Memo length limit (256).
    /// 5. Amount > 0.
    pub fn validate_stateless(tx: &SignedTransaction) -> Result<(), String> {
        // 1. Check Amount
        if tx.transaction.amount == 0 {
            return Err("Transaction amount must be greater than 0".to_string());
        }

        // 1.1 Check Timestamp
        if tx.transaction.timestamp == 0 {
            return Err("Transaction timestamp must be set".to_string());
        }

        // 2. Check Memo Length
        if let Some(memo) = &tx.transaction.memo {
            if memo.len() > 256 {
                return Err("Memo too long (max 256 bytes)".to_string());
            }
        }

        // 3. Verify Public Key Format (32 bytes)
        if tx.public_key.len() != 32 {
            return Err("Invalid public key length (must be 32 bytes)".to_string());
        }

        // 4. Check 'From' Address Derivation
        // The 'from' address MUST be the Bech32 'nbex' encoding of the signer's public key.
        // Base58 (NodeID) is NOT allowed in the Ledger.
        
        use crate::address::address::Address; 
        
        // Note: verifying_key is derived around line 45/66 usually, but we need it here.
        // We will move the derivation up or scope it.
        let verifying_key = VerifyingKey::from_bytes(tx.public_key.as_slice().try_into().unwrap_or(&[0u8;32]))
            .map_err(|_| "Invalid public key bytes".to_string())?;

        // 4. Verify 'From' Address matches Public Key
        // We must detect the HRP (nbex or nbhd) from the provided 'from' address to verify correctly.
        // If we force 'nbex', we break Hidden Address ('nbhd') sending.
        let hrp = if tx.transaction.from.starts_with("nbhd") {
            "nbhd"
        } else {
            "nbex" // Default/Standard
        };

        let expected_address = Address::address_from_pk(&verifying_key, hrp)
             .map_err(|e| format!("Failed to derive address: {:?}", e))?;

        if tx.transaction.from != expected_address {
             // We can provide a helpful error if they sent Base58 by mistake
             if !tx.transaction.from.starts_with("nbex") && !tx.transaction.from.starts_with("nbhd") {
                 let expected_base58 = bs58::encode(&tx.public_key).into_string();
                 if tx.transaction.from == expected_base58 {
                     return Err("Invalid 'from' address. Ledger requires Bech32 (nbex... or nbhd...), but got Base58 (NodeID).".to_string());
                 }
             }
             return Err(format!("Signature Mismatch! The Public Key provided derives to [{}], but the transaction claims to be from [{}]. Check if you are signing with the correct wallet profile.", expected_address, tx.transaction.from));
        }

        // 5. Check 'To' Address Format
        // Must be valid Bech32 ('nbex' OR 'nbhd') UNLESS it is a system address.
        if !tx.transaction.to.starts_with("system:") && tx.transaction.to != "mint" {
             if !tx.transaction.to.starts_with("nbex") && !tx.transaction.to.starts_with("nbhd") {
                 return Err(format!("Invalid 'to' address format. Ledger requires 'nbex' or 'nbhd' (Bech32), got: {}", tx.transaction.to));
             }

             // Validate checksum and length via Address utility
             use crate::address::address::Address;
             if !Address::is_valid(&tx.transaction.to) {
                  return Err(format!("Invalid 'to' address: Checksum failed or invalid format."));
             }
        }

        // 6. Verify Signature
        let verifying_key = VerifyingKey::from_bytes(tx.public_key.as_slice().try_into().unwrap())
            .map_err(|_| "Invalid public key bytes".to_string())?;
        
        // We assume signature is 64 bytes
        if tx.signature.len() != 64 {
             return Err("Invalid signature length".to_string());
        }
        let signature = Signature::from_slice(&tx.signature)
             .map_err(|_| "Invalid signature format".to_string())?;
        
        let msg = signing_bytes(&tx.transaction);
        verifying_key.verify(&msg, &signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        Ok(())
    }
}
