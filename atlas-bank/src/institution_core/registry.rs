use std::collections::HashMap;
use super::institution::Institution;
use atlas_common::error::Result;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstitutionRegistry {
    institutions: HashMap<String, Institution>,
}

impl InstitutionRegistry {
    pub fn new() -> Self {
        Self {
            institutions: HashMap::new(),
        }
    }

    pub fn add_institution(&mut self, institution: Institution) -> Result<()> {
        if self.institutions.contains_key(&institution.id) {
             return Err(atlas_common::error::AtlasError::Other(format!("Institution already exists: {}", institution.id)));
        }
        self.institutions.insert(institution.id.clone(), institution);
        Ok(())
    }

    pub fn get_institution(&self, id: &str) -> Option<&Institution> {
        self.institutions.get(id)
    }

    pub fn is_authorized(&self, id: &str) -> bool {
        self.institutions.contains_key(id)
    }
}
