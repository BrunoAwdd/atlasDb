use atlas_common::{
    error::{Result, AtlasError},
    transactions::{Transaction},
    entry::{Leg, LegKind, LedgerEntry},
};
use crate::core::ledger::state::State;

// Define Action Type
pub type InterceptorAction = Box<dyn FnOnce(&mut State) -> std::result::Result<(), String> + Send>;

pub struct InterceptorHandler;

impl InterceptorHandler {
    
    pub fn handle_registry(entry: &mut LedgerEntry, tx: &Transaction) -> Result<Option<InterceptorAction>> {
        if tx.to == "system:registry" {
             // Enforce Payment Asset is ATLAS
             if tx.asset != "ATLAS" {
                 return Err(AtlasError::Other("Registration fee must be paid in ATLAS".to_string()));
             }

             // Enforce Fee: 100 ATLAS
             let registration_fee: u64 = 100;
             if tx.amount < registration_fee as u128 {
                  return Err(AtlasError::Other("Insufficient registration fee (100 ATLAS required)".to_string()));
             }
             
             // Move funds from "system:registry" (where AccountingEngine put them) to "vault:fees"
             entry.legs.push(Leg {
                 account: "wallet:system:registry".to_string(),
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Debit, 
                 amount: registration_fee as u128,
             });
             entry.legs.push(Leg {
                 account: "vault:fees".to_string(),
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Credit,
                 amount: registration_fee as u128,
             });

             if let Some(memo) = &tx.memo {
                 if let Ok(asset_def) = serde_json::from_str::<crate::core::ledger::asset::AssetDefinition>(memo) {
                     // Enforce issuer matches sender (Simplistic check, ideally check signature of issuer)
                     if asset_def.issuer != tx.from {
                         return Err(AtlasError::Other("Issuer mismatch: You can only register assets for yourself/your institution".to_string()));
                     }
                     
                     // Validate strict definition
                     if let Err(e) = asset_def.validate() {
                         return Err(AtlasError::Other(format!("Invalid Asset Definition: {}", e)));
                     }

                     let asset_id = asset_def.id();
                     tracing::info!("®️ REGISTER ASSET: {} ({}) by {}", asset_id, asset_def.name, tx.from);

                     let action = Box::new(move |state: &mut State| {
                         // STRICT: Issuer must be a valid Institution in atlas-bank registry
                         if !state.institutions.is_authorized(&asset_def.issuer) {
                             return Err(format!("Issuer '{}' is not a authorized Institution in Atlas Bank", asset_def.issuer));
                         }

                         if state.assets.contains_key(&asset_id) {
                             return Err(format!("Asset {} already registered", asset_id));
                         }
                         state.assets.insert(asset_id, asset_def);
                         Ok(())
                     });
                     
                     return Ok(Some(action));
                 } else {
                     return Err(AtlasError::Other("Invalid AssetDefinition JSON in Memo".to_string()));
                 }
             } else {
                  return Err(AtlasError::Other("Missing Metadata in Memo".to_string()));
             }
        }
        Ok(None)
    }

    pub fn handle_staking(entry: &mut LedgerEntry, tx: &Transaction) -> Result<Option<InterceptorAction>> {
        if tx.to == "system:staking" {
             if let Some(memo) = &tx.memo {
                 if memo.starts_with("delegate:") {
                     // Memo: delegate:<VALIDATOR_ADDRESS>
                     let parts: Vec<&str> = memo.split(':').collect();
                     if parts.len() >= 2 {
                         let validator = parts[1].to_string();
                         let amount = tx.amount as u64;
                         let delegator = tx.from.clone();
                         
                         tracing::info!("🤝 DELEGATE: {} delegating {} to {}", delegator, amount, validator);
                         
                         let action = Box::new(move |state: &mut State| {
                             state.delegations.delegate(delegator, validator, amount);
                             Ok(())
                         });
                         return Ok(Some(action));
                     }
                 } else if memo.starts_with("undelegate:") {
                     // Memo: undelegate:<VALIDATOR_ADDRESS>:<AMOUNT>
                     let parts: Vec<&str> = memo.split(':').collect();
                     if parts.len() >= 3 {
                         let validator = parts[1].to_string();
                         if let Ok(amount) = parts[2].parse::<u64>() {
                             let delegator = tx.from.clone();
                             
                             tracing::info!("🤝 UNDELEGATE: {} withdrawing {} from {}", delegator, amount, validator);
                             
                             // 1. Queue State Update (Reduce Delegation)
                             let action = Box::new(move |state: &mut State| {
                                 state.delegations.undelegate(delegator, validator, amount)
                             });

                             // 2. Add Refund Legs (Pool -> User)
                             entry.legs.push(Leg {
                                 account: "wallet:system:staking".to_string(), // Debiting Pool (Corrected)
                                 asset: "ATLAS".to_string(),
                                 kind: LegKind::Debit, // Pool pays out
                                 amount: amount as u128,
                             });
                             entry.legs.push(Leg {
                                 account: format!("wallet:{}", tx.from),
                                 asset: "ATLAS".to_string(),
                                 kind: LegKind::Credit, // User receives
                                 amount: amount as u128,
                             });
                             
                             return Ok(Some(action));
                         }
                     }
                 }
             }
        }
        Ok(None)
    }
}
