use std::sync::Arc;

use tokio::sync::{RwLock};
use atlas_sdk::{
    auth::Authenticator, 
    utils::NodeId
};

use crate::{
    env::runtime::AtlasEnv, 
    Cluster, 
};

pub struct ClusterBuilder {
    env: Option<AtlasEnv>,
    auth: Option<Arc<RwLock<dyn Authenticator>>>,
    node_id: Option<NodeId>,
}

impl ClusterBuilder {
    pub fn new() -> Self {
        Self {
            env: None,
            node_id: None,
            auth: None,
        }
    }

    pub fn with_env(mut self, env: AtlasEnv) -> Self {
        self.env = Some(env);
        self
    }

    pub fn with_node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn with_auth(mut self, auth: Arc<RwLock<dyn Authenticator>>) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn build(self) -> Result<Cluster, String> {
        let env = self.env.ok_or("Missing env")?;
        let node_id = self.node_id.ok_or("Missing node_id")?;
        let auth = self.auth.ok_or("Missing auth")?;

        let cluster = Cluster::new(
            env, 
            node_id,
            auth
        );

        Ok(cluster)
    }
}

impl Default for ClusterBuilder {
    fn default() -> Self {
        Self::new()
    }
}