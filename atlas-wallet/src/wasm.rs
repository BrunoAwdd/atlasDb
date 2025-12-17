use js_sys::Uint8Array;
use serde_json::json;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;

use crate::wallet::{create_vault as create_vault_default, Wallet};

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Creates a new Wallet instance and returns a pointer to it.
/// The caller is responsible for freeing the memory with `wallet_free`.
#[wasm_bindgen]
pub fn wallet_new() -> usize {
    let wallet = Box::new(Wallet::new());
    Box::into_raw(wallet) as usize
}

/// Frees the memory of a Wallet instance.
#[wasm_bindgen]
pub fn wallet_free(wallet_ptr: usize) {
    if wallet_ptr == 0 {
        return;
    }
    unsafe {
        let _ = Box::from_raw(wallet_ptr as *mut Wallet);
    }
}

// `create_vault` remains a free function as it doesn't operate on a Wallet instance.
#[wasm_bindgen]
pub fn create_vault(
    password: String, 
) -> Result<JsValue, JsValue> {
    let (encrypted, exposed_address, hidden_address) = create_vault_default(password)?;
    
    let response = json!({
        "encrypted": encrypted,
        "exposed_address": exposed_address,
        "hidden_address": hidden_address,
    });

    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&format!("Erro de serialização: {:?}", e)))
}

#[wasm_bindgen]
pub fn load_vault(wallet_ptr: usize, password: String, encoded: JsValue) -> Result<(), JsValue> {
    let wallet = unsafe { &mut *(wallet_ptr as *mut Wallet) };
    let array = Uint8Array::new(&encoded);
    let mut encoded_vec = vec![0u8; array.length() as usize];
    array.copy_to(&mut encoded_vec);

    wallet.load_vault(password, encoded_vec).map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}

#[wasm_bindgen]
pub fn get_data(wallet_ptr: usize) -> Result<JsValue, JsValue> {
    let wallet = unsafe { &*(wallet_ptr as *mut Wallet) };
    let wallet_data = wallet.get_data()?;

    serde_wasm_bindgen::to_value(&wallet_data)
        .map_err(|e| JsValue::from_str(&format!("Erro de serialização: {:?}", e)))
}

#[wasm_bindgen]
pub fn sing_transfer(
    wallet_ptr: usize,
    to_address: String,
    amount: u64,
    password: String,
    memo: Option<String>,
) -> Result<JsValue, JsValue> {
    let wallet = unsafe { &mut *(wallet_ptr as *mut Wallet) };
    let (id,request) = wallet.sing_transfer(to_address, amount, password, memo)?;

    let payload = serde_json::json!({
        "id": id,
        "transfer": request
    });

    let serialized = serde_wasm_bindgen::to_value(&payload)
        .map_err(|e| JsValue::from_str(&format!("Erro ao serializar: {:?}", e)))?;

    Ok(serialized)
}


#[wasm_bindgen]
pub fn switch_profile(wallet_ptr: usize) -> Result<(), JsValue> {
    let wallet = unsafe { &mut *(wallet_ptr as *mut Wallet) };
    wallet.switch_profile().map_err(|e| JsValue::from_str(&e))?;

    Ok(())
}
