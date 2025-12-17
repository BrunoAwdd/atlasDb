use std::convert::TryFrom;
use std::fmt;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use super::address::Address;
use super::errors::AddressError;
use super::profile_address::ProfileAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HiddenAddress(Address);

impl TryFrom<String> for HiddenAddress {
    type Error = AddressError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with("nbhd") {
            Ok(HiddenAddress(Address::try_from(s)?))
        } else {
            Err(AddressError::InvalidPublicKey("Expected prefix 'nbhd'".to_string()))
        }
    }
}

impl TryFrom<&str> for HiddenAddress {
    type Error = AddressError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.starts_with("nbhd") {
            Ok(HiddenAddress(Address::try_from(s)?))
        } else {
            Err(AddressError::InvalidPublicKey("Expected prefix 'nbhd'".to_string()))
        }
    }
}

impl fmt::Display for HiddenAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


impl HiddenAddress {
    pub fn new(address: Address) -> Result<Self, AddressError> {
        if address.as_str().starts_with("nbhd") {
            Ok(HiddenAddress(address))
        } else {
            Err(AddressError::InvalidPublicKey("Expected 'nbhd' prefix".into()))
        }
    }

    pub fn from_public_key(pk: &VerifyingKey) -> Result<Self, AddressError> {
        let raw = Address::address_from_pk(pk, "nbhd")?;
        HiddenAddress::new(Address::try_from(raw)?)
    }

    pub fn inner(&self) -> &Address {
        &self.0
    }
}

impl ProfileAddress for HiddenAddress {
    fn address(&self) -> &Address {
        &self.0
    }

    fn prefix(&self) -> &'static str {
        "nbhd"
    }
}