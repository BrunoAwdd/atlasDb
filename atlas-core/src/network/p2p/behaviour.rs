use libp2p::{
    gossipsub::{Behaviour as GossipsubBehaviour},
    identify::{Behaviour as IdentifyBehaviour},
    kad::{store::MemoryStore, Behaviour as KademliaBehaviour},
    ping::{Behaviour as PingBehaviour},
    request_response::{Behaviour as RequestResponseBehaviour},
    swarm::{NetworkBehaviour},
};

use super::{
    codec::TxCodec,
    error::P2pError,
};

// DICA: ajuste o caminho do ComposedEvent conforme seu layout real.
// Se o módulo é "events.rs" no mesmo nível deste arquivo, use `super::events::ComposedEvent`.
// Se for em `crate::network::p2p::events`, use esse caminho completo.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "super::events::ComposedEvent", event_process = false)]
pub struct P2pBehaviour {
    pub identify: IdentifyBehaviour,
    pub ping: PingBehaviour,
    #[cfg(feature = "mdns")]
    pub mdns: libp2p::mdns::tokio::Behaviour,
    pub kad: KademliaBehaviour<MemoryStore>,
    pub gossipsub: GossipsubBehaviour,
    pub rr: RequestResponseBehaviour<TxCodec>, // seu codec define Req/Resp
}

impl P2pBehaviour {
    pub fn subscribe_core_topics(&mut self) -> Result<(), P2pError> {
        use libp2p::gossipsub::IdentTopic;

        let topics = [
            IdentTopic::new("atlas/heartbeat/v1"),
            IdentTopic::new("atlas/proposal/v1"),
            IdentTopic::new("atlas/vote/v1"),
        ];

        for t in topics {
            match self.gossipsub.subscribe(&t) {
                Ok(_)  => {
                    tracing::debug!("gossipsub subscribed -> {}", t.hash());
                    println!("gossipsub subscribed -> {}", t.hash());
                },
                Err(e) => tracing::error!("gossipsub subscribe FAILED -> {}: {e}", t.hash()),
            }
        }
        Ok(())
    }
}