use ed25519_dalek::{
    Signature, 
    Signer,
    SigningKey,
    VerifyingKey, 
    Verifier
};

use atlas_common::{address::address::{Address, TypedAddress}, address::profile_address::ProfileAddress, transactions::{errors::TransactionError, payload::TransferPayload, TransferRequest}};
use serde::{Serialize, Deserialize};
use crate::{errors::NimbleError, identity::{errors::IdentityError, secret::decrypt_secret_key}};
use crate::profile::{hidden::HiddenProfile, exposed::ExposedProfile};

pub enum Profile {
    Hidden(HiddenProfile),
    Exposed(ExposedProfile),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    #[serde(with = "crate::identity::serde_pubkey")]
    pub public_key: VerifyingKey,

    #[serde(default)] 
    encrypted_sk: Option<String>,

    pub id: String,
    pub address: TypedAddress,
    pub permissions: Vec<String>,
    pub transactions: Vec<String>,
    pub salt: [u8; 16],
    pub is_public: bool,
}

impl ProfileData {
    pub fn new(
        id: String,
        public_key: VerifyingKey,
        encrypted_sk: Option<String>,
        address: TypedAddress,
        permissions: Vec<String>,
        salt: [u8; 16],
        is_public: bool,
    ) -> Self {
        Self {
            id,
            public_key,
            encrypted_sk,
            address,
            permissions,
            transactions: vec![],
            salt,
            is_public,
        }
    }

    pub fn add_payload(&mut self, to: &Address, amount: u64, nonce: u64) {
        let payload = TransferPayload::new(&self.address.as_str(), to.as_str().to_string(), amount, nonce);
        self.transactions.push(payload.to_string());
    }

    pub fn build_signed_request<'a>(
        &'a self,
        to: &'a Address,
        amount: u64,
        password: String,
        memo: Option<&str>,
        nonce: u64,
    ) -> Result<TransferRequest, NimbleError> {
        let payload = TransferPayload::new(&self.address.as_str(), to.as_str().to_string(), amount, nonce);
        let secret = self.get_signing_key(password)?;

        let sig2 = self.sign(secret, payload.to_string().as_bytes())?;

        let request = TransferRequest::build_signed_request(
            self.address.as_str().to_string(),
            to.as_str().to_string(),
            amount,
            sig2,
            memo.map(|m| m.to_string()),
            payload.timestamp,
            nonce,
        )?;

        Ok(request)
    }

    pub fn sign(&self, secret: SigningKey, message: &[u8]) -> Result<[u8; 64], IdentityError> {
        let signature_bytes = {
            let signing_key = SigningKey::from(secret);
            signing_key.sign(message).to_bytes()
        };
    
        Ok(signature_bytes)
    }

    pub fn get_signing_key(&self, encryption_key: String) -> Result<SigningKey, IdentityError> {
        let encrypted = self.encrypted_sk.clone()
            .ok_or(IdentityError::InvalidPrivateKey("missing key".into()))?;

        let secret = decrypt_secret_key(&encrypted, encryption_key, self.salt)?;

        Ok(secret)
    }


    pub fn validate_transfer(&self, request: TransferRequest, public_key: VerifyingKey) -> Result<(), NimbleError> {
        let payload = request.to_payload();

        if !payload.is_valid() {
            return Err(TransactionError::InvalidPayload("Invalid payload".into()).into());
        }

        Self::validate_msg(&payload.to_string().as_bytes().to_vec(), &request.sig2, public_key)?;
   
        Ok(())
    }

    pub fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64], public_key: VerifyingKey) -> Result<(), NimbleError> {
        Self::validate_msg(&msg, signature, public_key)
    }

    fn validate_msg(
        msg: &Vec<u8>,
        signature: &[u8; 64],
        public_key: VerifyingKey
    ) -> Result<(), NimbleError> {
        let sig = Signature::from_bytes(signature);

        public_key
            .verify(msg, &sig)
            .map_err(|e| TransactionError::InvalidPasswordSignature(e.to_string()).into())
    }

    pub fn validate_permissions(&self, permission: String) -> Result<(), IdentityError> {
        if !self.permissions.contains(&permission) {
            return Err(IdentityError::InvalidPermission(
                format!("Profile '{}' does not have permission '{}'", self.id, permission)
            ));
        }
        Ok(())
    }

    pub fn try_validate_permissions(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    pub fn zeroize(&mut self) {
        use zeroize::Zeroize;
        self.encrypted_sk.zeroize();
        self.salt.zeroize();
    }

    pub fn is_cleared(&self) -> bool {
        match &self.encrypted_sk {
            Some(sk) => sk.is_empty(),
            None => true,
        }
    }

    pub fn is_public(&self) -> bool {
        self.is_public
    }

}

pub trait ProfileType {
    fn new_from_seed(
        seed: &[u8; 32],
        password: String, 
        label: &str, 
        permissions: Vec<String>
    ) -> Result<Self, IdentityError> where Self: Sized;

    fn add_payload(&mut self, to: &Address, amount: u64, nonce: u64);

    fn build_signed_request<'a>(
        &'a self,
        to: &'a Address,
        amount: u64,
        password: String,
        memo: Option<&str>,
        nonce: u64,
    ) -> Result<TransferRequest, NimbleError>;
    fn get_signing_key(&self, encryption_key: String) -> Result<SigningKey, IdentityError>;

    fn sign(&self, encryption_key: SigningKey, message: &[u8]) -> Result<[u8; 64], IdentityError>;

    fn validate_transfer(&self, request: TransferRequest, public_key: VerifyingKey) -> Result<(), NimbleError>;

    fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64], public_key: VerifyingKey) -> Result<(), NimbleError> ;

    fn validate_permissions(&self, permission: String) -> Result<(), IdentityError>;

    fn try_validate_permissions(&self, permission: &str) -> bool;

    fn zeroize(&mut self);

    fn is_cleared(&self) -> bool;

    fn is_public(&self) -> bool;
}

impl Profile {
    pub fn address(&self) -> &TypedAddress {
        match self {
            Profile::Hidden(p) => p.address(),
            Profile::Exposed(p) => p.address(),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Profile::Hidden(p) => p.id(),
            Profile::Exposed(p) => p.id(),
        }
    }
}

impl ProfileType for Profile {
    fn new_from_seed(
        _seed: &[u8; 32],
        _password: String,
        _label: &str,
        _permissions: Vec<String>,
    ) -> Result<Self, IdentityError> {
        unimplemented!("Use HiddenProfile or ExposedProfile directly for creation")
    }

    fn add_payload(&mut self, to: &Address, amount: u64, nonce: u64) {
        match self {
            Profile::Hidden(p) => p.add_payload(to, amount, nonce),
            Profile::Exposed(p) => p.add_payload(to, amount, nonce),
        }
    }

    fn build_signed_request<'a>(
        &'a self,
        to: &'a Address,
        amount: u64,
        password: String,
        memo: Option<&str>,
        nonce: u64,
    ) -> Result<TransferRequest, NimbleError> {
        match self {
            Profile::Hidden(p) => p.build_signed_request(to, amount, password, memo, nonce),
            Profile::Exposed(p) => p.build_signed_request(to, amount, password, memo, nonce),
        }
    }

    fn get_signing_key(&self, encryption_key: String) -> Result<SigningKey, IdentityError> {
        match self {
            Profile::Hidden(p) => p.get_signing_key(encryption_key),
            Profile::Exposed(p) => p.get_signing_key(encryption_key),
        }
    }

    fn sign(&self, secret: SigningKey, message: &[u8]) -> Result<[u8; 64], IdentityError> {
        match self {
            Profile::Hidden(p) => p.sign(secret, message),
            Profile::Exposed(p) => p.sign(secret, message),
        }
    }

    fn validate_transfer(&self, request: TransferRequest, public_key: VerifyingKey) -> Result<(), NimbleError> {
        match self {
            Profile::Hidden(p) => p.validate_transfer(request, public_key),
            Profile::Exposed(p) => p.validate_transfer(request, public_key),
        }
    }

    fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64], public_key: VerifyingKey) -> Result<(), NimbleError> {
        match self {
            Profile::Hidden(p) => p.validate_message(msg, signature, public_key),
            Profile::Exposed(p) => p.validate_message(msg, signature, public_key),
        }
    }

    fn validate_permissions(&self, permission: String) -> Result<(), IdentityError> {
        match self {
            Profile::Hidden(p) => p.validate_permissions(permission),
            Profile::Exposed(p) => p.validate_permissions(permission),
        }
    }

    fn try_validate_permissions(&self, permission: &str) -> bool {
        match self {
            Profile::Hidden(p) => p.try_validate_permissions(permission),
            Profile::Exposed(p) => p.try_validate_permissions(permission),
        }
    }

    fn zeroize(&mut self) {
        match self {
            Profile::Hidden(p) => p.zeroize(),
            Profile::Exposed(p) => p.zeroize(),
        }
    }

    fn is_cleared(&self) -> bool {
        match self {
            Profile::Hidden(p) => p.is_cleared(),
            Profile::Exposed(p) => p.is_cleared(),
        }
    }

    fn is_public(&self) -> bool {
        match self {
            Profile::Hidden(p) => p.is_public(),
            Profile::Exposed(p) => p.is_public(),
        }
    }
}