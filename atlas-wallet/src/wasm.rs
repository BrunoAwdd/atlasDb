use js_sys::Uint8Array;
use serde_json::json;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;

use crate::wallet::{create_vault as create_vault_default, Wallet};

use std::cell::RefCell;

thread_local! {
    static WALLET: RefCell<Wallet> = RefCell::new(Wallet::new());
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

// `create_vault` remains a free function
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
pub fn load_vault(password: String, encoded: JsValue) -> Result<(), JsValue> {
    let array = Uint8Array::new(&encoded);
    let mut encoded_vec = vec![0u8; array.length() as usize];
    array.copy_to(&mut encoded_vec);

    WALLET.with(|wallet| {
        wallet.borrow_mut().load_vault(password, encoded_vec).map_err(|e| JsValue::from_str(&e))
    })
}

#[wasm_bindgen]
pub fn get_data() -> Result<JsValue, JsValue> {
    WALLET.with(|wallet| {
        let wallet_data = wallet.borrow().get_data()?;
        serde_wasm_bindgen::to_value(&wallet_data)
            .map_err(|e| JsValue::from_str(&format!("Erro de serialização: {:?}", e)))
    })
}

#[wasm_bindgen]
pub fn sing_transfer(
    to_address: String,
    amount: u64,
    memo: Option<String>,
) -> Result<JsValue, JsValue> {
    WALLET.with(|wallet| {
        let mut w = wallet.borrow_mut();
        let (id, request, public_key) = w.sing_transfer(to_address, amount, memo)?;

        let payload = serde_json::json!({
            "id": id,
            "transfer": {
                "signature": hex::encode(request.sig2),
                "public_key": public_key,
                "transaction": request
            }
        });

        serde_wasm_bindgen::to_value(&payload)
            .map_err(|e| JsValue::from_str(&format!("Erro ao serializar: {:?}", e)))
    })
}

#[wasm_bindgen]
pub fn switch_profile() -> Result<(), JsValue> {
    WALLET.with(|wallet| {
        wallet.borrow_mut().switch_profile().map_err(|e| JsValue::from_str(&e))
    })
}
