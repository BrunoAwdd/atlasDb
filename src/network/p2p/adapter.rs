use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::peer_manager::{PeerManager, PeerCommand, PeerEvent};
use crate::utils::NodeId;
use crate::cluster::node::Node;

use super::{
    behaviour::P2pBehaviour as Behaviour,
    config::P2pConfig,
    events::{AdapterEvent, ComposedEvent},
    protocol::TxRequest,
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
    identity, 
    kad, 
    noise, 
    request_response::{
        Behaviour as RequestResponseBehaviour, 
        Config as RequestResponseConfig, 
        Event as RequestResponseEvent, 
        Message,
        OutboundRequestId as RequestId, 
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
    StreamProtocol, 
    Transport
};
use tokio::sync::mpsc;

pub enum AdapterCmd {
    Publish { topic: String, data: Vec<u8> },
    RequestTxs { peer: libp2p::PeerId, req: TxRequest },
    Shutdown,
}
pub struct Libp2pAdapter {
    pub peer_id: PeerId,
    pub swarm: Swarm<Behaviour>,
    pub evt_tx: mpsc::Sender<AdapterEvent>,
    cmd_rx: mpsc::Receiver<AdapterCmd>,
    peer_mgr: PeerManager,
    addr_book: HashMap<NodeId, HashSet<Multiaddr>>,
    dial_backoff: HashMap<NodeId, Instant>,
    last_kad_bootstrap: std::time::Instant,   
}


impl Libp2pAdapter {
    pub async fn new(cfg: P2pConfig, evt_tx: mpsc::Sender<AdapterEvent>, cmd_rx: mpsc::Receiver<AdapterCmd>) -> Result<Self, P2pError> {
        // chave/peer id
        let key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(key.public());

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

        let peer_mgr = PeerManager::new(/*max_active*/ 8, /*max_reserve*/ 64);
        let addr_book = HashMap::new();
        let dial_backoff = HashMap::new();
        let last_kad_bootstrap = std::time::Instant::now();

        Ok(Self { peer_id, swarm, evt_tx, cmd_rx, peer_mgr, addr_book, dial_backoff, last_kad_bootstrap })
    }

    /// Loop principal: processa eventos do Swarm e repassa ao Cluster
    pub async fn run(mut self) {
        use futures::StreamExt;
        let mut maintain = tokio::time::interval(Duration::from_secs(10));
        
    
        loop {
            tokio::select! {
                // 1) eventos do swarm
                swarm_ev = self.swarm.select_next_some() => {
                    match swarm_ev {
                        SwarmEvent::Behaviour(ComposedEvent::Identify(ev)) => {
                            if let libp2p::identify::Event::Received { peer_id, info, .. } = ev {
                                let id = peer_id.into();
                                for addr in info.listen_addrs {
                                    self.learn_addr(&id, addr.clone());
                                    self.swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                                }
                                // toque o peer (marca last_seen = agora)
                                self.touch_peer(id);
                            
                                if self.last_kad_bootstrap.elapsed() >= Duration::from_secs(60) {
                                    let _ = self.swarm.behaviour_mut().kad.bootstrap();
                                    self.last_kad_bootstrap = std::time::Instant::now();
                                }
                            }
                        }
    
                        SwarmEvent::Behaviour(ComposedEvent::Ping(ev)) => {
                            if let libp2p::ping::Event { peer, result: Ok(rtt), .. } = ev {
                                let id: NodeId = peer.into();
                                // atualiza latência e last_seen
                                let mut n = self
                                    .peer_mgr
                                    .get_peer_stats(&id)
                                    .unwrap_or_else(|| Node::placeholder());
                                n.update_latency(Some(rtt.as_millis() as u64));
                                n.update_last_seen();
                                let _ = self.peer_mgr.handle_command(PeerCommand::UpdateStats(id, n));
                            }
                        }
    
                        #[cfg(feature = "mdns")]
                        SwarmEvent::Behaviour(ComposedEvent::Mdns(ev)) => {
                            match ev {
                                libp2p::mdns::Event::Discovered(list) => {
                                    for (peer, addr) in list {
                                        let id: NodeId = peer_id.into();
                                        self.learn_addr(&id, addr.clone());
                                        self.swarm.behaviour_mut().kad.add_address(&peer, addr.clone());
                                        let node = Node { reliability_score: 0.0, latency: None, ..Default::default() };
                                        let _ = self.peer_mgr.handle_command(PeerCommand::Register(id.clone(), node));
                                        let _ = Swarm::dial(&mut self.swarm, addr);
                                        let _ = self.evt_tx.send(AdapterEvent::PeerDiscovered(peer)).await;
                                    }
                                }
                                libp2p::mdns::Event::Expired(list) => {
                                    for (peer, addr) in list {
                                        let id: NodeId = peer_id.into();
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
                                let id: NodeId = peer.into();
                                for addr in addresses.into_vec() {
                                    self.learn_addr(&id, addr.clone());
                                    let _ = Swarm::dial(&mut self.swarm, addr);
                                }
                                let _ = self.evt_tx.send(AdapterEvent::PeerDiscovered(peer)).await;
                            }
                        }
    
                        SwarmEvent::Behaviour(ComposedEvent::Gossipsub(ev)) => {
                            match ev {
                                GossipsubEvent::Message { propagation_source, message, .. } => {
                                    let topic = message.topic.as_str().to_string();
                                    let data  = message.data.clone();
                                    tracing::info!("RX gossipsub topic={} size={} from={}", topic, data.len(), propagation_source);
                                    // repassa para evt_tx
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
                                    let id: NodeId = peer.into();
                                    self.touch_peer(id);
                                    let _ = (request, channel);
                                    // self.swarm.behaviour_mut().rr.send_response(channel, resp)?;
                                }
                                Message::Response { response, .. } => {
                                    let id: NodeId = peer.into();
                                    self.touch_peer(id);
                                    let _ = response;
                                }
                            },
                        
                            // novas variantes (cubra com .. para estabilidade):
                            RequestResponseEvent::OutboundFailure { peer, .. } => {
                                // Ex.: timeout, conexão fechada, dial failure...
                                // Marque atividade mínima do peer; você pode também registrar métrica/decair score.
                                let id: NodeId = peer.into();
                                self.touch_peer(id);
                                // log::debug!("RR outbound failure: {:?}", error); // se quiser capturar o erro, inclua `error`
                            }
                            RequestResponseEvent::InboundFailure { peer, .. } => {
                                let id: NodeId = peer.into();
                                self.touch_peer(id);
                                // log::debug!("RR inbound failure: {:?}", error);
                            }
                            RequestResponseEvent::ResponseSent { peer, .. } => {
                                // resposta enviada com sucesso
                                let id: NodeId = peer.into();
                                self.touch_peer(id);
                            }
                        
                            _ => {}
                        },
                        
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            let id: NodeId = peer_id.into();
                            if !self.peer_mgr.known_peers.contains_key(&id) {
                                let mut node = Node::placeholder();
                                node.update_last_seen();
                                let _ = self.peer_mgr.handle_command(PeerCommand::Register(id, node));
                            } else {
                                self.touch_peer(id);
                            }
                        }
    
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            let id = peer_id.into();
                            let _ = self.peer_mgr.handle_command(PeerCommand::Disconnected(id));
                        }
    
                        _ => {}
                    }
                }
    
                // 2) manutenção (braço separado!)
                _ = maintain.tick() => {
                    // disca alguns da reserva (que não estejam ativos)
                    let active = self.peer_mgr.get_active_peers();
                    let reserve: Vec<NodeId> = self
                        .peer_mgr
                        .get_reserve_peers()
                        .into_iter()
                        .filter(|id| !active.contains(id))
                        .take(8)
                        .collect();
    
                    for id in reserve {
                        self.try_dial_with_backoff(&id);
                    }
    
                    // rotação (no máx 1 swap por chamada)
                    let _ = self.peer_mgr.handle_command(PeerCommand::Rotate);
    
                    // bootstrap do KAD com cooldown de 60s
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
                                    // opcional: re-enfileirar/retry depois de um pequeno delay
                                    let _ = self.evt_tx.send(AdapterEvent::PublishFailed { topic: t.to_string(), data }).await;
                                }
                            }
                        }
                        Some(AdapterCmd::RequestTxs { peer, req }) => {
                            let _ = self.swarm.behaviour_mut().rr.send_request(&peer, req);
                        }
                        Some(AdapterCmd::Shutdown) | None => break,
                    }
                }
            }
        }
    }
    
    fn touch_peer(&mut self, id: NodeId) {
        let mut n = self
            .peer_mgr
            .get_peer_stats(&id)
            .unwrap_or_else(|| Node::placeholder());
        n.update_last_seen();
        let _ = self.peer_mgr.handle_command(PeerCommand::UpdateStats(id, n));
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
                let _ = Swarm::dial(&mut self.swarm, addr);
            }
            self.dial_backoff.insert(id.clone(), now + Duration::from_secs(30));
        }
    }
}
