use std::sync::Arc;
use std::time::Duration;

use crate::{
    cluster::core::Cluster, 
    cluster_proto::{
        HeartbeatMessage, 
        ProposalMessage, 
        VoteMessage
    },
    NodeId, 
    Proposal,
};

#[derive(Debug, Clone)]
pub enum ClusterCommand {
    // Operations
    AddProposal(Proposal),
    BroadcastHeartbeat,
    BroadcastProposals,
    SendVote,

    // Decisions
    EvaluateProposals,
    CommitProposal(String),
    RejectProposal(String),

    // Handle (ingest)
    HandleHeartbeat(HeartbeatMessage),
    HandleProposal(ProposalMessage),
    HandleVote(VoteMessage),

    // State
    SyncState(NodeId),
    GossipState,

    // Shutdown
    Shutdown,    
}

impl ClusterCommand {
    /// Timeout padrão por comando (ajuste à vontade)
    fn timeout(&self) -> Duration {
        match self {
            ClusterCommand::AddProposal(_)      => Duration::from_secs(30),
            ClusterCommand::BroadcastHeartbeat  => Duration::from_secs(30),
            ClusterCommand::BroadcastProposals  => Duration::from_secs(30),
            ClusterCommand::HandleHeartbeat(_)  => Duration::from_secs(10),
            ClusterCommand::HandleProposal(_)   => Duration::from_secs(10),
            ClusterCommand::HandleVote(_)       => Duration::from_secs(10),
            ClusterCommand::SendVote            => Duration::from_secs(10),
            ClusterCommand::EvaluateProposals   => Duration::from_secs(15),
            ClusterCommand::CommitProposal(_)   => Duration::from_secs(15),
            ClusterCommand::RejectProposal(_)   => Duration::from_secs(15),
            ClusterCommand::SyncState(_)        => Duration::from_secs(20),
            ClusterCommand::GossipState         => Duration::from_secs(20),
            ClusterCommand::Shutdown            => Duration::from_secs(5),
        }
    }

    pub async fn execute(self, cluster: &Arc<Cluster>) -> Result<(), String> {
        match self {
            // --- Operations ---
            ClusterCommand::AddProposal(proposal) => 
                cluster.add_proposal(proposal).await,
            ClusterCommand::BroadcastHeartbeat => 
                cluster.broadcast_heartbeats().await,
            ClusterCommand::BroadcastProposals => 
                cluster.broadcast_proposals().await,
            ClusterCommand::SendVote => 
                cluster.vote_proposals().await,

            // --- Decisions ---
            ClusterCommand::EvaluateProposals => {
                cluster.evaluate_proposals().await
            }
            ClusterCommand::CommitProposal(id) => {
                // Self::with_timeout(self.timeout(),
                //     async move { cluster.commit_proposal(&id).await },
                //     "Timeout while committing proposal",
                // ).await
                Ok(())
            }
            ClusterCommand::RejectProposal(id) => {
                // Self::with_timeout(self.timeout(),
                //     async move { cluster.reject_proposal(&id).await },
                //     "Timeout while rejecting proposal",
                // ).await
                Ok(())
            }

            // --- Handle (ingest) ---
            ClusterCommand::HandleHeartbeat(msg) => {
                cluster.handle_heartbeat(msg).await.map(|_| ())
            }
            ClusterCommand::HandleProposal(msg) => {
                cluster.handle_proposal(msg).await.map(|_| ())
            }
            ClusterCommand::HandleVote(msg) => {
                cluster.handle_vote(msg).await.map(|_| ())
            }

            // --- State ops ---
            ClusterCommand::SyncState(_peer) => {
                // Self::with_timeout(self.timeout(),
                //     async move { cluster.sync_state_with(peer).await },
                //     "Timeout while syncing state",
                // ).await
                Ok(())
            }
            ClusterCommand::GossipState => {
                // Self::with_timeout(self.timeout(),
                //     async move { cluster.gossip_state().await },
                //     "Timeout while gossiping state",
                // ).await
                Ok(())
            }

            // --- Shutdown ---
            ClusterCommand::Shutdown => {
                cluster.shutdown_grpc().await;
                Ok(())
            }
        }
    }
}
