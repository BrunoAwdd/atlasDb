use ed25519_dalek::VerifyingKey;

use super::address::{Address, TypedAddress};
use super::errors::AddressError;

pub trait ProfileAddress {
    fn address(&self) -> &Address;

    fn public_key(&self) -> Result<VerifyingKey, AddressError> {
        Address::public_key_from_str(self.address().as_str())
    }

    fn prefix(&self) -> &'static str;

    fn as_str(&self) -> &str {
        self.address().as_str()
    }
}

impl ProfileAddress for TypedAddress {
    fn address(&self) -> &Address {
        match self {
            TypedAddress::Hidden(h) => h.address(),
            TypedAddress::Exposed(e) => e.address(),
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            TypedAddress::Hidden(h) => h.prefix(),
            TypedAddress::Exposed(e) => e.prefix(),
        }
    }
}
