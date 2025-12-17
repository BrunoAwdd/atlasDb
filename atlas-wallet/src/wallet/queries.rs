use super::{Wallet, WalletData, AccountData};
use atlas_common::transactions::TransferRequest;
use atlas_common::address::profile_address::ProfileAddress;

pub(super) fn get_data(wallet: &Wallet) -> Result<WalletData, String> {
    let session = wallet.session.as_ref().ok_or_else(|| "Sessão não carregada".to_string())?;

    let hidden_address = session.identity.hidden.address().as_str();
    let exposed_address = session.identity.exposed.address().as_str();
    let balance = 100;

    let wallet_data = WalletData {
        exposed: AccountData {
            address: exposed_address.to_string(),
            balance: balance,
        },
        hidden: AccountData {
            address: hidden_address.to_string(),
            balance: balance,
        },
    };

    Ok(wallet_data)
}

pub(super) fn selected_account(wallet: &Wallet) -> Result<String, String> {
    let session = wallet.session.as_ref().ok_or_else(|| "Sessão não carregada".to_string())?;
    Ok(session.profile.address().as_str().to_string())
}

pub(super) fn validade(wallet: &Wallet, transfer: TransferRequest) -> Result<String, String> {
    let session = wallet.session.as_ref().ok_or_else(|| "Sessão não carregada".to_string())?;
    session.validate_transfer(transfer.clone())
        .map(|_| "Transferência válida".to_string())
        .map_err(|e| format!("Erro ao validar transferência: {:?}", e))
}

pub(super) fn validate_message(wallet: &Wallet, msg: Vec<u8>, signature: &[u8; 64]) -> Result<String, String> {
    let session = wallet.session.as_ref().ok_or_else(|| "Sessão não carregada".to_string())?;
    session.validate_message(msg, signature)
        .map(|_| "Transferência válida".to_string())
        .map_err(|e| format!("Erro ao validar transferência: {:?}", e))
}
