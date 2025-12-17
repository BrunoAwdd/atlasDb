use rand::{rngs::OsRng, RngCore};

pub fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn generate_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_salt_length() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16, "Salt should be exactly 16 bytes");
    }

    #[test]
    fn test_generate_salt_randomness() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2, "Salts generated consecutively should not be equal");
    }

    #[test]
    fn test_generate_nounce_length() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 12, "Nounce should be exactly 12 bytes");
    }

    #[test]
    fn test_generate_nounce_randomness() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();
        assert_ne!(nonce1, nonce2, "Nounces generated consecutively should not be equal");
    }

    #[test]
    fn test_seed_length() {
        let seed = generate_seed();
        assert_eq!(seed.len(), 32, "Seed should be exactly 32 bytes");
    }

    #[test]
    fn test_seed_randomness() {
        let seed1 = generate_seed();
        let seed2 = generate_seed();
        assert_ne!(seed1, seed2, "Seeds generated consecutively should not be equal");
    }

    #[test]
    fn test_seed_is_deterministic() {
        let seed1 = generate_seed();
        let seed2 = generate_seed();
        assert_ne!(seed1, seed2, "Seeds generated consecutively should be equal");
    }


}
