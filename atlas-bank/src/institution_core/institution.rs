use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Institution {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub compliance_tier: String, // e.g., "Tier1", "Tier2"
}

impl Institution {
    pub fn new(id: String, name: String, public_key: String, compliance_tier: String) -> Self {
        Self {
            id,
            name,
            public_key,
            compliance_tier,
        }
    }
}
