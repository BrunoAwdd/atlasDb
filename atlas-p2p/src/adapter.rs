use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use atlas_common::utils::NodeId;
use crate::config::P2pConfig;
use crate::events::{AdapterEvent, ComposedEvent};
use crate::peer_manager::{PeerManager, PeerCommand};
// use crate::traits::NetworkAdapter;

use atlas_common::env::node::Node;

use crate::protocol::{
    TxRequest,
};

use crate::{
    behaviour::P2pBehaviour as Behaviour,
    error::P2pError,
};

use libp2p::{
    core::upgrade, 
    gossipsub::{
        self, 
        IdentTopic,
        MessageAuthenticity, 
        ValidationMode,
        Event as GossipsubEvent,
    }, 
    identify, 
    kad, 
    noise, 
    request_response::{
        Behaviour as RequestResponseBehaviour, 
        Config as RequestResponseConfig, 
        Event as RequestResponseEvent, 
        Message,
        OutboundRequestId, 
        ProtocolSupport
    }, 
    swarm::{
        Config as SwarmConfig, 
        Swarm, 
        SwarmEvent
    }, 
    tcp, 
    yamux, 
    Multiaddr, 
    PeerId, 
    Transport,
    StreamProtocol,
};

type RequestId = OutboundRequestId;

use tokio::sync::{mpsc, RwLock};

use crate::key_manager;
use std::path::Path;

pub struct Libp2pAdapter {
    pub peer_id: PeerId,
    pub swarm: Swarm<Behaviour>,
    pub evt_tx: mpsc::Sender<AdapterEvent>,
    cmd_rx: mpsc::Receiver<AdapterCmd>,
    peer_mgr: Arc<RwLock<PeerManager>>,
    addr_book: HashMap<NodeId, HashSet<Multiaddr>>,
    dial_backoff: HashMap<NodeId, Instant>,
    last_kad_bootstrap: std::time::Instant,
    pending_responses: HashMap<u64, libp2p::request_response::ResponseChannel<crate::protocol::TxBundle>>,
    next_req_id: u64,
}

pub enum AdapterCmd {
    Publish { topic: String, data: Vec<u8> },
    RequestTxs { peer: libp2p::PeerId, req: TxRequest },
    SendResponse { req_id: u64, res: crate::protocol::TxBundle },
    Shutdown,
}



impl Libp2pAdapter {
    pub async fn new(cfg: P2pConfig, evt_tx: mpsc::Sender<AdapterEvent>, cmd_rx: mpsc::Receiver<AdapterCmd>, peer_mgr: Arc<RwLock<PeerManager>>) -> Result<Self, P2pError> {
        // chave/peer id
        let key = key_manager::load_or_generate_keypair(Path::new(&cfg.keypair_path))
            .map_err(P2pError::Io)?;
        let peer_id = PeerId::from(key.public());

        // ... (rest of the function is the same)

        // transporte
        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1Lazy)
            .authenticate(noise::Config::new(&key)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // gossipsub
        let gcfg = gossipsub::ConfigBuilder::default()
            .validation_mode(ValidationMode::Strict)
            .build()
            .unwrap();

        let gs = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(key.clone()),
            gcfg,
        ).map_err(P2pError::GossipsubInit)?;

        // identify
        let identify = identify::Behaviour::new(
            identify::Config::new("atlas/1.0".into(), key.public())
                .with_agent_version("rust-libp2p".into())
        );

        // mdns
        #[cfg(feature = "mdns")]
        let mdns = libp2p::mdns::tokio::Behaviour::new(
            libp2p::mdns::Config::default(), peer_id
        )?;

        // kad
        let mut kad_cfg = kad::Config::default();
        kad_cfg.set_query_timeout(std::time::Duration::from_secs(5));
        let store = kad::store::MemoryStore::new(peer_id);
        let kad = kad::Behaviour::with_config(peer_id, store, kad_cfg);

        // request-response
        let rr = {
            let mut cfg = RequestResponseConfig::default();
            #[allow(deprecated)]
            cfg.set_request_timeout(std::time::Duration::from_secs(3));
        
            let protocols = std::iter::once((
                StreamProtocol::new("/atlas/tx/1"),
                ProtocolSupport::Full,
            ));
        
            // Antes: RequestResponseBehaviour::new(TxCodec, protocols, cfg)
            // Agora:
            RequestResponseBehaviour::new(protocols, cfg) // TCodec = TxCodec (inference)
        };

        let mut behaviour = Behaviour {
            identify,
            ping: libp2p::ping::Behaviour::default(),
            #[cfg(feature = "mdns")]
            mdns,
            kad,
            gossipsub: gs,
            rr,
        };

        // tópicos
        behaviour.subscribe_core_topics()?; // usa P2pError::Gossipsub

        // swarm
        let mut swarm = Swarm::new(transport, behaviour, peer_id, SwarmConfig::with_tokio_executor());

        // listen
        for ma in &cfg.listen_multiaddrs {
            Swarm::listen_on(&mut swarm, ma.parse::<Multiaddr>()?)?;
        }

        // bootstrap
        for b in &cfg.bootstrap {
            if let Ok(addr) = b.parse::<Multiaddr>() {
                Swarm::dial(&mut swarm, addr)?;
            }
        }

        let mut addr_book = HashMap::new();
        {
            let pm = peer_mgr.read().await;
            for (id, node) in &pm.known_peers {
                if let Ok(ma) = node.address.parse::<Multiaddr>() {
                    addr_book.entry(id.clone()).or_insert_with(HashSet::new).insert(ma);
                }
            }
        }
        let dial_backoff = HashMap::new();
        let last_kad_bootstrap = std::time::Instant::now();
        let pending_responses = HashMap::new();
        let next_req_id = 0;

        Ok(Self { peer_id, swarm, evt_tx, cmd_rx, peer_mgr, addr_book, dial_backoff, last_kad_bootstrap, pending_responses, next_req_id })
    }

    /// Loop principal: processa eventos do Swarm e repassa ao Cluster
    pub async fn run(mut self) {
        use libp2p::futures::StreamExt;
        let mut maintain = tokio::time::interval(Duration::from_secs(10));
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(3));
        
    
        loop {
            tokio::select! {
                // 1) eventos do swarm
                swarm_ev = self.swarm.select_next_some() => {
                    match swarm_ev {
                        SwarmEvent::Behaviour(ComposedEvent::Identify(ev)) => {
                            if let libp2p::identify::Event::Received { peer_id, info, .. } = ev {
                                let id = peer_id.to_string().into();
                                for addr in info.listen_addrs {
                                    self.learn_addr(&id, addr.clone());
                                    self.swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                                }
                                // toque o peer (marca last_seen = agora)
                                self.touch_peer(id).await;
                            
                                if self.last_kad_bootstrap.elapsed() >= Duration::from_secs(60) {
                                    let _ = self.swarm.behaviour_mut().kad.bootstrap();
                                    self.last_kad_bootstrap = std::time::Instant::now();
                                }
                            }
                        }
    
                        SwarmEvent::Behaviour(ComposedEvent::Ping(ev)) => {
                            if let libp2p::ping::Event { peer, result: Ok(rtt), .. } = ev {
                                let id: NodeId = peer.to_string().into();
                                // atualiza latência e last_seen
                                let mut peer_mgr = self.peer_mgr.write().await;
                                let mut n = peer_mgr
                                    .get_peer_stats(&id)
                                    .unwrap_or_else(|| Node::placeholder());
                                n.update_latency(Some(rtt.as_millis() as u64));
                                n.update_last_seen();
                                let _ = peer_mgr.handle_command(PeerCommand::UpdateStats(id, n));
                            }
                        }
    
                        #[cfg(feature = "mdns")]
                        SwarmEvent::Behaviour(ComposedEvent::Mdns(ev)) => {
                            match ev {
                                libp2p::mdns::Event::Discovered(list) => {
                                    for (peer, addr) in list {
                                        tracing::info!("🔍 mDNS Discovered: {} at {}", peer, addr);
                                        let id: NodeId = peer.to_string().into();
                                        self.learn_addr(&id, addr.clone());
                                        self.swarm.behaviour_mut().kad.add_address(&peer, addr.clone());
                                        let mut node = Node::placeholder();
                                        node.reliability_score = 0.0;
                                        node.latency = None;
                                        self.peer_mgr.write().await.handle_command(PeerCommand::Register(id.clone(), node));
                                        let _ = Swarm::dial(&mut self.swarm, addr);
                                        if let Ok(_) = self.evt_tx.send(AdapterEvent::PeerDiscovered(peer.to_string().into())).await {
                                            // Handle error if necessary
                                        }
                                    }
                                }
                                libp2p::mdns::Event::Expired(list) => {
                                    for (peer, addr) in list {
                                        let id: NodeId = peer.to_string().into();
                                        self.swarm.behaviour_mut().kad.remove_address(&peer, &addr);
                                        if let Some(set) = self.addr_book.get_mut(&id) {
                                            set.remove(&addr);
                                            if set.is_empty() { self.addr_book.remove(&id); }
                                        }
                                    }
                                }
                            }
                        }
    
                        SwarmEvent::Behaviour(ComposedEvent::Kad(ev)) => {
                            if let kad::Event::RoutingUpdated { peer, addresses, .. } = ev {
                                let id: NodeId = peer.to_string().into();
                                for addr in addresses.into_vec() {
                                    self.learn_addr(&id, addr.clone());
                                    let _ = Swarm::dial(&mut self.swarm, addr);
                                }
                                if let Ok(_) = self.evt_tx.send(AdapterEvent::PeerDiscovered(peer.to_string().into())).await {
                                    // Handle error if necessary
                                }
                            }
                        }
    
                        SwarmEvent::Behaviour(ComposedEvent::Gossipsub(ev)) => {
                            match ev {
                                GossipsubEvent::Message { propagation_source, message, .. } => {
                                    let topic = message.topic.as_str();
                                    let data = message.data.clone();
                                    let from = message.source.unwrap_or(propagation_source);
                                    tracing::info!("RX gossipsub topic={} size={} from={}", topic, data.len(), from);

                                    let event = match topic {
                                        "atlas/heartbeat/v1" => AdapterEvent::Heartbeat {
                                            from: from.to_string().into(),
                                            data,
                                        },
                                        "atlas/proposal/v1" => AdapterEvent::Proposal(data),
                                        "atlas/vote/v1" => AdapterEvent::Vote(data),
                                        _ => AdapterEvent::Gossip {
                                            topic: topic.to_string(),
                                            from: from.to_string().into(),
                                            data,
                                        },
                                    };

                                    if let Err(e) = self.evt_tx.send(event).await {
                                        tracing::error!("evt_tx send error: {e}");
                                    }
                                }
                                GossipsubEvent::Subscribed { peer_id, topic } => {
                                    tracing::info!("peer {peer_id} subscribed to {}", topic.as_str());
                                }
                                GossipsubEvent::Unsubscribed { peer_id, topic } => {
                                    tracing::info!("peer {peer_id} unsubscribed from {}", topic.as_str());
                                }
                                _ => {}
                            }
                        }
                        
    
                        SwarmEvent::Behaviour(ComposedEvent::ReqRes(ev)) => match ev {
                            RequestResponseEvent::Message { peer, message, .. } => match message {
                                Message::Request { request, channel, .. } => {
                                    // atividade do peer
                                    let id: NodeId = peer.to_string().into();
                                    self.touch_peer(id.clone()).await;
                                    
                                    let req_id = self.next_req_id;
                                    self.next_req_id += 1;
                                    self.pending_responses.insert(req_id, channel);

                                    if let Err(e) = self.evt_tx.send(AdapterEvent::TxRequest { from: id, req: request, req_id }).await {
                                        tracing::error!("evt_tx send error: {e}");
                                    }
                                }
                                Message::Response { response, .. } => {
                                    let id: NodeId = peer.to_string().into();
                                    self.touch_peer(id.clone()).await;
                                    // Handle response (TxBundle)
                                    if let Err(e) = self.evt_tx.send(AdapterEvent::TxBundle { from: id, bundle: response }).await {
                                        tracing::error!("evt_tx send error: {e}");
                                    }
                                }
                            },
                        
                            // novas variantes (cubra com .. para estabilidade):
                            RequestResponseEvent::OutboundFailure { peer, .. } => {
                                let id: NodeId = peer.to_string().into();
                                self.touch_peer(id).await;
                            }
                            RequestResponseEvent::InboundFailure { peer, .. } => {
                                let id: NodeId = peer.to_string().into();
                                self.touch_peer(id).await;
                            }
                            RequestResponseEvent::ResponseSent { peer, .. } => {
                                let id: NodeId = peer.to_string().into();
                                self.touch_peer(id).await;
                            }
                        
        
                        },
                        
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            let id: NodeId = peer_id.to_string().into();
                            let mut peer_mgr = self.peer_mgr.write().await;
                            if !peer_mgr.known_peers.contains_key(&id) {
                                let mut node = Node::placeholder();
                                node.update_last_seen();
                                let _ = peer_mgr.handle_command(PeerCommand::Register(id, node));
                            } else {
                                // Drop the lock before calling touch_peer, which will lock it again
                                drop(peer_mgr);
                                self.touch_peer(id).await;
                            }
                        }
    
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            let id = peer_id.to_string().into();
                            self.peer_mgr.write().await.handle_command(PeerCommand::Disconnected(id));
                        }

                        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                            tracing::warn!("❌ Outgoing connection error to {:?}: {:?}", peer_id, error);
                        }
                        SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, connection_id: _ } => {
                            tracing::warn!("❌ Incoming connection error from {:?} to {:?}: {:?}", send_back_addr, local_addr, error);
                        }
                        ev => {
                            tracing::debug!("Unhandled Swarm Event: {:?}", ev);
                        }
                    }
                }
    
                // 2) manutenção (braço separado!)
                _ = heartbeat_interval.tick() => {
                    let topic = IdentTopic::new("atlas/heartbeat/v1");
                    let data = b"hi from adapter".to_vec();
                    // println!("💓 heartbeat");
                    if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                        tracing::warn!("Failed to publish heartbeat: {e}");
                    }
                }

                _ = maintain.tick() => {
                    let peer_mgr = self.peer_mgr.read().await;
                    let active = peer_mgr.get_active_peers();
                    let reserve: Vec<NodeId> = peer_mgr
                        .get_reserve_peers()
                        .into_iter()
                        .filter(|id| !active.contains(id))
                        .take(8)
                        .collect();
                    
                    // Drop the read lock before acquiring a write lock
                    drop(peer_mgr);
    
                    for id in reserve {
                        self.try_dial_with_backoff(&id);
                    }
    
                    self.peer_mgr.write().await.handle_command(PeerCommand::Rotate);
    
                    if self.last_kad_bootstrap.elapsed() >= Duration::from_secs(60) {
                        let _ = self.swarm.behaviour_mut().kad.bootstrap();
                        self.last_kad_bootstrap = std::time::Instant::now();
                    }
                }

                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(AdapterCmd::Publish { topic, data }) => {
                            let t = IdentTopic::new(&topic);
                            match self.swarm.behaviour_mut().gossipsub.publish(t.clone(), data.clone()) {
                                Ok(id) => {
                                    tracing::info!("TX gossipsub ok topic={} id={id}", t.hash().to_string());
                                }
                                Err(e) => {
                                    tracing::warn!("TX gossipsub FAIL topic={} err={e}", t.hash().to_string());
                                    if let Ok(_) = self.evt_tx.send(AdapterEvent::PublishFailed { topic: t.to_string(), data }).await {
                                        // Handle error if necessary
                                    }
                                }
                            }
                        }
                        Some(AdapterCmd::RequestTxs { peer, req }) => {
                            let _ = self.swarm.behaviour_mut().rr.send_request(&peer, req);
                        }
                        Some(AdapterCmd::SendResponse { req_id, res }) => {
                            if let Some(channel) = self.pending_responses.remove(&req_id) {
                                let _ = self.swarm.behaviour_mut().rr.send_response(channel, res);
                            } else {
                                tracing::warn!("SendResponse: unknown req_id {}", req_id);
                            }
                        }
                        Some(AdapterCmd::Shutdown) | None => break,
                    }
                }
            }
        }
    }
    
    async fn touch_peer(&mut self, id: NodeId) {
        let mut peer_mgr = self.peer_mgr.write().await;
        let mut n = peer_mgr
            .get_peer_stats(&id)
            .unwrap_or_else(|| Node::placeholder());
        n.update_last_seen();
        let _ = peer_mgr.handle_command(PeerCommand::UpdateStats(id, n));
    }
    

    // helpers p/ publicar e request/response
    pub fn publish(&mut self, topic: &str, bytes: Vec<u8>) {
        let t = IdentTopic::new(topic);
        let _ = self.swarm.behaviour_mut().gossipsub.publish(t, bytes);
    }

    pub fn request_txs(&mut self, peer: libp2p::PeerId, req: TxRequest) -> RequestId {
        self.swarm.behaviour_mut().rr.send_request(&peer, req)
    }

    fn learn_addr(&mut self, id: &NodeId, addr: Multiaddr) {
        self.addr_book.entry(id.clone()).or_default().insert(addr);
    }

    fn try_dial_with_backoff(&mut self, id: &NodeId) {
        // backoff simples: 30s por peer
        let now = Instant::now();
        if let Some(next_ok) = self.dial_backoff.get(id) {
            if now < *next_ok { return; }
        }
        if let Some(addrs) = self.addr_book.get(id) {
            for addr in addrs.iter().cloned() {
                match Swarm::dial(&mut self.swarm, addr) {
                    Ok(_) => tracing::info!("📞 Dialing {}", id),
                    Err(e) => tracing::warn!("❌ Dial failed for {}: {}", id, e),
                }
            }
            self.dial_backoff.insert(id.clone(), now + Duration::from_secs(5));
        }
    }
}
