use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serializer, Deserializer};
use serde::de::Error;

pub fn serialize<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(key.as_bytes())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = <Vec<u8>>::deserialize(deserializer)?;
    let array: [u8; 32] = bytes.try_into().map_err(|_| D::Error::custom("Invalid length"))?;
    VerifyingKey::from_bytes(&array).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{VerifyingKey, SigningKey};
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    struct Dummy {
        #[serde(with = "super")]
        pub_key: VerifyingKey,
    }

    fn fixed_keypair() -> (SigningKey, VerifyingKey) {
        let sk = SigningKey::from_bytes(&[42u8; 32]);
        let pk = VerifyingKey::from(&sk);
        (sk, pk)
    }

    #[test]
    fn test_serialize_deserialize_public_key() {
        let (_sk, pk) = fixed_keypair();

        let original = Dummy { pub_key: pk };

        let serialized = bincode::serialize(&original).expect("Serialization failed");
        let deserialized: Dummy = bincode::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(original.pub_key, deserialized.pub_key, "Original and deserialized public keys must match");
    }

    #[test]
    fn test_serialized_length_is_correct() {
        let (_sk, pk) = fixed_keypair();

        let serialized = bincode::serialize(&Dummy { pub_key: pk }).expect("Serialization failed");

        assert!(serialized.len() >= 32, "Serialized size should include at least 32 bytes plus overhead");
    }

    #[test]
    fn test_deserialization_from_invalid_data() {
        let bad_data = bincode::serialize(&vec![1, 2, 3, 4]).unwrap();
        let result: Result<Dummy, _> = bincode::deserialize(&bad_data);
        assert!(result.is_err(), "Should fail to deserialize invalid public key bytes");
    }
}
