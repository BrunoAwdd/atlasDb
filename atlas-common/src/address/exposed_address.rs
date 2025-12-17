
use std::convert::TryFrom;
use std::fmt;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use super::address::Address;
use super::errors::AddressError;
use super::profile_address::ProfileAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExposedAddress(Address);

impl TryFrom<String> for ExposedAddress {
    type Error = AddressError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with("nbex") {
            Ok(ExposedAddress(Address::try_from(s)?))
        } else {
            Err(AddressError::InvalidPublicKey("Expected prefix 'nbex'".to_string()))
        }
    }
}

impl TryFrom<&str> for ExposedAddress {
    type Error = AddressError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.starts_with("nbex") {
            Ok(ExposedAddress(Address::try_from(s)?))
        } else {
            Err(AddressError::InvalidPublicKey("Expected prefix 'nbex'".to_string()))
        }
    }
}

impl fmt::Display for ExposedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ExposedAddress {
    pub fn new(address: Address) -> Result<Self, AddressError> {
        if address.as_str().starts_with("nbex") {
            Ok(ExposedAddress(address))
        } else {
            Err(AddressError::InvalidPublicKey("Expected 'nbex' prefix".into()))
        }
    }

    pub fn from_public_key(pk: &VerifyingKey) -> Result<Self, AddressError> {
        let raw = Address::address_from_pk(pk, "nbex")?;
        ExposedAddress::new(Address::try_from(raw)?)
    }

    pub fn inner(&self) -> &Address {
        &self.0
    }
}

impl ProfileAddress for ExposedAddress {
    fn address(&self) -> &Address {
        &self.0
    }

    fn prefix(&self) -> &'static str {
        "nbex"
    }
}