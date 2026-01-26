use atlas_ledger::Ledger;
use atlas_common::env::proposal::Proposal;
use atlas_common::entry::{Leg, LegKind, LedgerEntry};
use tokio::runtime::Runtime;
use atlas_ledger::core::ledger::schema::{AccountSchema, AccountClass, Liquidity};

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        println!("🚀 Starting Hybrid Ledger Verification...");
        
        let data_dir = "/tmp/atlas_hybrid_demo";
        let _ = std::fs::remove_dir_all(data_dir); // Clean start
        std::fs::create_dir_all(data_dir).unwrap();

        // 1. Initialize Ledger (Monolith + Shards)
        println!("📦 Initializing Ledger in: {}", data_dir);
        let ledger = Ledger::new(data_dir).await.expect("Failed to init ledger");

        // 2. Fund Alice (Genesis Simulation)
        println!("\n🏛️  Step 1: Genesis Funding");
        {
            let mut state = ledger.state.write().await;
            // "0xAlice": Hash-based Wallet (Implicitly 2.1 Liability)
            let account = state.accounts.entry("0xAlice".to_string())
                .or_insert_with(atlas_ledger::core::ledger::account::AccountState::new);
            account.balances.insert("ATLAS".to_string(), 1000);
            println!("   -> Alice Balance: 1000 ATLAS (0xAlice)");
        }

        // 3. Transaction: Alice -> Bob
        println!("\n💸 Step 2: Alice sends 50 ATLAS to Bob");
        let proposal_json = r#"{
            "from": "0xAlice", 
            "to": "0xBob", 
            "amount": 50, 
            "asset": "ATLAS", 
            "nonce": 1,
            "memo": "Hash Transfer",
            "signature": [],
            "public_key": []
        }"#;

        // Mock Proposal Wrapper
        let proposal = Proposal {
            id: "prop-1".to_string(),
            proposer: atlas_common::utils::NodeId("validator-1".to_string()),
            content: proposal_json.to_string(),
            parent: None,
            height: 1,
            signature: [0; 64],
            public_key: vec![],
            hash: "hash_prop_1".to_string(),
            prev_hash: "hash_genesis".to_string(),
            round: 1,
            state_root: "".to_string(),
            time: 1234567890,
        };

        // Execute! 
        println!("   ... Executing Proposal ...");
        match ledger.execute_transaction(&proposal, true).await {
            Ok(count) => println!("   ✅ Success! Executed {} transaction(s).", count),
            Err(e) => panic!("   ❌ Execution Failed: {}", e),
        }

        // 4. Verify State (Memory)
        println!("\n🧠 Step 3: Verifying RAM State (Monolith)");
        let alice_bal = ledger.get_balance("0xAlice", "ATLAS").await.unwrap();
        let bob_bal = ledger.get_balance("0xBob", "ATLAS").await.unwrap();
        println!("   -> Alice (0xAlice) New Balance: {} (Expected 950)", alice_bal);
        println!("   -> Bob   (0xBob)   New Balance: {} (Expected 50)", bob_bal);
        
        assert_eq!(alice_bal, 950);
        assert_eq!(bob_bal, 50);

        // 5. Verify Shards (Disk) - Flat Check
        println!("\n💾 Step 4: Verifying Physical Shards");
        let alice_file = std::path::Path::new(data_dir).join("accounts").join("0xAlice.bin");
        let bob_file = std::path::Path::new(data_dir).join("accounts").join("0xBob.bin");

        if alice_file.exists() {
            println!("   ✅ Alice's Shard Found: {:?}", alice_file);
        } else {
            println!("   ❌ Alice's Shard MISSING!");
        }

        if bob_file.exists() {
            println!("   ✅ Bob's Shard Found: {:?}", bob_file);
        } else {
            println!("   ❌ Bob's Shard MISSING!");
        }

        // 6. Verify Schema Logic (Mixed)
        println!("\n📚 Step 5: Verifying Mixed Schema Logic");
        
        // Test Case A: Standard Internal Account
        let internal_acc = "4.1:Vendas";
        let class = AccountSchema::parse_root(internal_acc);
        let liquidity = AccountSchema::get_liquidity(internal_acc);
        println!("   -> Account '{}': Class={}, Liquidity={:?}", internal_acc, class, liquidity);
        assert_eq!(class, AccountClass::Resultado);
        assert_eq!(liquidity, Liquidity::None);

        // Test Case B: Wallet Address
        let wallet_acc = "0xAlice";
        let w_class = AccountSchema::parse_root(wallet_acc);
        let w_liq = AccountSchema::get_liquidity(wallet_acc);
        println!("   -> Account '{}': Class={}, Liquidity={:?}", wallet_acc, w_class, w_liq);
        assert_eq!(w_class, AccountClass::Passivo, "Wallets must be Liability");
        assert_eq!(w_liq, Liquidity::Current, "Wallets must be Current Liability");

        // Test Case C: Strict Validation
        let valid = AccountSchema::validate("4.1:Vendas");
        let invalid = AccountSchema::validate("9.9:Fake");
        println!("   -> Validation Check: 4.1:Vendas={}, 9.9:Fake={}", valid, invalid);
        assert!(valid);
        assert!(!invalid);
        
        println!("\n🎉 Verification Complete! Core Engine + Schema Logic Operational.");
    });
}
