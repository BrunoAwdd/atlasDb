use libp2p::{
    gossipsub,
    identify,
    kad,
    request_response,
    ping,
    PeerId,
};

use atlas_sdk::utils::NodeId;

use crate::network::p2p::protocol::{TxRequest, TxBundle};


#[derive(Debug)]
pub enum ComposedEvent {
    Identify(IdentifyEvent),
    Ping(ping::Event),
    #[cfg(feature = "mdns")]
    Mdns(libp2p::mdns::Event),
    Kad(kad::Event),
    Gossipsub(GossipsubEvent),
    ReqRes(RequestResponseEvent<TxRequest, TxBundle>),
}

use gossipsub::Event as GossipsubEvent;
use identify::Event as IdentifyEvent;
use request_response::Event as RequestResponseEvent;

impl From<IdentifyEvent> for ComposedEvent { fn from(e: IdentifyEvent) -> Self { Self::Identify(e) } }
impl From<ping::Event>     for ComposedEvent { fn from(e: ping::Event)     -> Self { Self::Ping(e) } }
#[cfg(feature = "mdns")]
impl From<libp2p::mdns::Event> for ComposedEvent { fn from(e: libp2p::mdns::Event) -> Self { Self::Mdns(e) } }
impl From<kad::Event> for ComposedEvent { fn from(e: kad::Event) -> Self { Self::Kad(e) } }
impl From<GossipsubEvent> for ComposedEvent { fn from(e: GossipsubEvent) -> Self { Self::Gossipsub(e) } }
impl From<RequestResponseEvent<TxRequest, TxBundle>> for ComposedEvent {
    fn from(e: RequestResponseEvent<TxRequest, TxBundle>) -> Self { Self::ReqRes(e) }
}

/// Eventos que o Adapter entrega para a camada superior (Cluster)
#[derive(Debug)]
pub enum AdapterEvent {
    PeerDiscovered(NodeId),
    Heartbeat { from: NodeId, data: Vec<u8> },
    Proposal(Vec<u8>),
    PublishFailed {topic: String, data: Vec<u8>},
    Gossip {topic: String, data: Vec<u8>, from: NodeId},
    Vote(Vec<u8>),
    TxRequest { from: NodeId, txids: Vec<[u8;32]> },
    TxBundle  { from: NodeId, txs: Vec<Vec<u8>> },
}
