use atlas_common::utils::security::generate_seed;
use crate::{
    identity::identity::generate, 
    vault::vault::VaultData,
};
use atlas_common::address::profile_address::ProfileAddress;

pub fn create_vault(
    password: String, 
) -> Result<(Vec<u8>, String, String), String> {
    let seed = generate_seed();
    let bundle = generate(&seed, password.clone())
                                    .expect("Failed to generate identity");
    let vault = VaultData::new(1, vec![0u8; 12]);

    let encrypted = vault
                                .save_identity_bundle(&bundle, password)
                                .expect("Failed to save bundle");

    Ok((
        encrypted, 
        bundle.identity.exposed.address().as_str().to_string(), 
        bundle.identity.hidden.address().as_str().to_string()
    ))
}
