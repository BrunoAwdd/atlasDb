use ed25519_dalek::VerifyingKey;
use bech32::{convert_bits, decode, encode, u5, FromBase32, Variant};
use serde::{Serialize, Deserialize};

use super::{
    errors::AddressError, 
    exposed_address::ExposedAddress, 
    hidden_address::HiddenAddress
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypedAddress {
    Hidden(HiddenAddress),
    Exposed(ExposedAddress),
}

impl TryFrom<&str> for TypedAddress {
    type Error = AddressError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.starts_with("nbhd") {
            Ok(TypedAddress::Hidden(HiddenAddress::try_from(s)?))
        } else if s.starts_with("nbex") {
            Ok(TypedAddress::Exposed(ExposedAddress::try_from(s)?))
        } else {
            Err(AddressError::InvalidPublicKey(format!("Unknown address prefix: {}", s)))
        }
    }
}


impl TryFrom<String> for Address {
    type Error = AddressError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if Address::is_valid(&s) {
            Ok(Address(s))
        } else {
            Err(AddressError::InvalidPublicKey(format!("Invalid address: {}", s)))
        }
    }
}

impl TryFrom<&str> for Address {
    type Error = AddressError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if Address::is_valid(s) {
            Ok(Address(s.to_string()))
        } else {
            Err(AddressError::InvalidPublicKey(format!("Invalid address: {}", s)))
        }
    }
}

impl TryFrom<String> for TypedAddress {
    type Error = AddressError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with("nbhd") {
            Ok(TypedAddress::Hidden(HiddenAddress::try_from(s)?))
        } else if s.starts_with("nbex") {
            Ok(TypedAddress::Exposed(ExposedAddress::try_from(s)?))
        } else {
            Err(AddressError::InvalidPublicKey(format!("Unknown address prefix: {}", s)))
        }
    }
}

impl std::ops::Deref for Address {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Address {
    /// Retorna se o endereço fornecido é válido.
    pub fn is_valid(address: &str) -> bool {
        Self::public_key_from_str(address).is_ok()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn handle_address(raw: &str) -> Result<TypedAddress, AddressError> {
        TypedAddress::try_from(raw)
    }

    pub fn is_public(&self) -> bool {
        self.0.starts_with("nbhd")
    }

    /// Extrai a chave pública de um endereço válido.
    pub fn public_key_from_str(address: &str) -> Result<VerifyingKey, AddressError> {
        let (hrp, data, variant) = decode(address)
            .map_err(|e| AddressError::InvalidPublicKey(e.to_string()))?;

        if (hrp != "nbex" && hrp != "nbhd") || variant != Variant::Bech32m {
            return Err(AddressError::InvalidPublicKey(format!("Invalid address: {}", address)));
        }

        let bytes = Vec::<u8>::from_base32(&data)
            .map_err(|e| AddressError::InvalidPublicKey(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(AddressError::InvalidPublicKeyLength(bytes.len()));
        }

        // Converte Vec<u8> para [u8; 32]
        let bytes_array: [u8; 32] = bytes.as_slice()
            .try_into()
            .map_err(|_| AddressError::InvalidPublicKey("Invalid public key length".to_string()))?;

        VerifyingKey::from_bytes(&bytes_array)
            .map_err(|e| AddressError::InvalidPublicKey(e.to_string()))
    }

    /// Converts a `VerifyingKey` into a bech32m-encoded address with the "nimble" prefix.
    ///
    /// The conversion involves:
    /// - Converting the 32-byte public key into 5-bit chunks (base32 compatible).
    /// - Encoding the result with the bech32m variant.
    ///
    /// # Panics
    ///
    /// This function will panic if the conversion to 5-bit or the final encoding fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// let address = address_from_pk(&public_key);
    /// assert!(address.starts_with("nimble1"));
    /// ```
    pub fn address_from_pk(public_key: &VerifyingKey, prefix: &str) -> Result<String, AddressError> {
        let bytes = public_key.to_bytes();

        let five_bit: Vec<u5> = convert_bits(&bytes, 8, 5, true)
            .map_err(AddressError::BitConversionFailed)?
            .into_iter()
            .map(|b| u5::try_from_u8(b).unwrap()) // `unwrap()` aqui ainda é seguro porque `convert_bits` garante que o valor é válido
            .collect();

        encode(prefix, five_bit, Variant::Bech32m)
            .map_err(|_| AddressError::EncodingFailed)
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use rand_core::OsRng;

    use super::*;

    pub fn seed() -> [u8; 32] {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        seed
    }

    /// Asserts that an address generated from a public key
    /// can be parsed back to the same public key.
    #[test]
    fn test_address_from_public_key_and_back() -> Result<(), AddressError> {
        // Usa seu próprio sistema de seed para gerar uma identidade
        let seed = seed();

        let secret_key = SigningKey::from_bytes(&seed);
        let public_key = VerifyingKey::from(&secret_key);

        // Gera o endereço
        let address_str = Address::address_from_pk(&public_key, "nbex")
            .map_err(|_| AddressError::InvalidPublicKey("Failed to generate address".to_string()))?;

        // Extrai a public key de volta a partir do address
        let extracted_pk = Address::public_key_from_str(&address_str)?;

        assert_eq!(public_key, extracted_pk);

        Ok(())
    }

    /// Verifies that an invalid address string is rejected.
    #[test]
    fn test_invalid_address_is_rejected() {
        let invalid_address = "nimble1invalidaddress";

        assert!(!Address::is_valid(invalid_address));
        assert!(Address::public_key_from_str(invalid_address).is_err());
    }

    /// Verifies that a random string that is not bech32 fails.
    #[test]
    fn test_completely_invalid_format_fails() {
        let random_string = "not_even_bech32_encoded";
        assert!(Address::try_from(random_string).is_err());
        assert!(Address::public_key_from_str(random_string).is_err());
    }
}
