
use ed25519_dalek::{SigningKey, VerifyingKey};
use atlas_common::{address::{address::TypedAddress, hidden_address::HiddenAddress}, transactions::TransferRequest};
use serde::{Serialize, Deserialize};
use crate::errors::NimbleError;
use crate::identity::errors::IdentityError;
use crate::identity::secret::encrypt_secret_key;

use atlas_common::{
    address::address::Address, 
    utils::security::generate_salt
};

use super::profile_type::{ProfileData, ProfileType};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenProfile {
    pub data: ProfileData,
}

impl HiddenProfile {
    pub fn address(&self) -> &TypedAddress {
        &self.data.address
    }

    pub fn id(&self) -> &str {
        &self.data.id
    }
}


impl ProfileType for HiddenProfile {
    fn new_from_seed(
        seed: &[u8; 32],
        password: String,
        label: &str,
        permissions: Vec<String>,
    ) -> Result<Self, IdentityError> {
        let secret = SigningKey::from_bytes(seed);
        let public = VerifyingKey::from(&secret);

        let address = TypedAddress::Hidden(HiddenAddress::from_public_key(&public)?);

        let salt = generate_salt();
        let encrypted = encrypt_secret_key(&secret, password, salt)?;

        Ok(HiddenProfile {
            data: ProfileData::new(
                label.to_string(),
                public,
                Some(encrypted),
                address,
                permissions,
                salt,
                false,
            ),
        })
    }

    fn add_payload(&mut self, to: &Address, amount: u64, nonce: u64) {
        self.data.add_payload(to, amount, nonce);
    }

    fn build_signed_request<'a>(
        &'a self,
        to: &'a Address,
        amount: u64,
        password: String,
        memo: Option<&str>,
        nonce: u64,
    ) -> Result<TransferRequest, NimbleError> {
        self.data.build_signed_request(to, amount, password, memo, nonce)
    }

    fn get_signing_key(&self, encryption_key: String) -> Result<SigningKey, IdentityError> {
        self.data.get_signing_key(encryption_key)
    }

    fn sign(&self, secret: SigningKey, message: &[u8]) -> Result<[u8; 64], IdentityError> {
        self.data.sign(secret, message)
    }

    fn validate_transfer(&self, request: TransferRequest, public_key: VerifyingKey) -> Result<(), NimbleError> {
        self.data.validate_transfer(request, public_key)
    }

    fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64], public_key: VerifyingKey) -> Result<(), NimbleError> {
        self.data.validate_message(msg, signature, public_key)
    }

    fn validate_permissions(&self, permission: String) -> Result<(), IdentityError> {
        self.data.validate_permissions(permission)
    }

    fn try_validate_permissions(&self, permission: &str) -> bool {
        self.data.try_validate_permissions(permission)
    }

    fn zeroize(&mut self) {
        self.data.zeroize();
    }

    fn is_cleared(&self) -> bool {
        self.data.is_cleared()
    }

    fn is_public(&self) -> bool {
        self.data.is_public
    }
}