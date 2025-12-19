use super::{Wallet, VaultData, Session};
use atlas_common::transactions::TransferRequest;

pub(super) fn load_vault(wallet: &mut Wallet, password: String, encoded: Vec<u8>) -> Result<(), String> {
    let vault = VaultData::new(1, vec![0u8; 12]);
    let bundle = vault
        .load_identity_bundle(password.clone(), encoded)
        .map_err(|e| format!("Erro ao descriptografar: {:?}", e))?;

    let session = Session::from_bundle(bundle)
        .map_err(|e| format!("Erro ao criar sessão: {:?}", e))?;

    wallet.session = Some(session);
    Ok(())
}

pub(super) fn sing_transfer(
    wallet: &mut Wallet,
    to_address: String,
    amount: u64,
    password: String,
    memo: Option<String>,
) -> Result<(String, TransferRequest, Vec<u8>), String> {
    let session = wallet.session.as_mut().ok_or_else(|| "Sessão não carregada".to_string())?;
    
    let nonce = (wallet.transfer_map.lock().map_err(|_| "Erro ao acessar mapa".to_string())?.len() as u64) + 1;

    let request = session.create_signed_transfer(
        to_address,
        amount,
        password,
        memo,
        nonce,
    ).map_err(|e| format!("Erro ao criar transferência: {:?}", e))?;

    let public_key = session.get_public_key().ok_or_else(|| "Erro ao obter chave pública".to_string())?;

    let id = hex::encode(&request.sig2);

    {
        let mut map = wallet.transfer_map.lock().map_err(|_| "Erro ao acessar mapa".to_string())?;
        map.insert(id.clone(), request.clone());
    }

    Ok((id, request, public_key))
}

pub(super) fn sign_message(wallet: &mut Wallet, message: Vec<u8>, password: String) -> Result<String, String> {
    let session = wallet.session.as_mut().ok_or_else(|| "Sessão não carregada".to_string())?;
    let slice: &[u8] = &message;

    session.unlock_key(password)
        .map_err(|e| format!("Erro ao desbloquear sessão: {:?}", e))?;

    let signature = session
        .sign_message(slice)
        .map_err(|e| format!("Erro ao assinar: {:?}", e))?;

    Ok(hex::encode(signature))
}

pub(super) fn switch_profile(wallet: &mut Wallet) -> Result<(), String> {
    let session = wallet.session.as_mut().ok_or_else(|| "Sessão não carregada".to_string())?;
    session.switch_profile().map_err(|e| format!("Erro ao trocar conta: {:?}", e))?;
    Ok(())
}
