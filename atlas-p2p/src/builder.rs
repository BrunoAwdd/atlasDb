use libp2p::{
    core::upgrade,
    gossipsub::{
        self,
        MessageAuthenticity,
        ValidationMode,
    },
    identify,
    kad,
    noise,
    request_response::{
        Behaviour as RequestResponseBehaviour,
        Config as RequestResponseConfig,
        ProtocolSupport,
    },
    swarm::{
        Config as SwarmConfig,
        Swarm,
    },
    tcp,
    yamux,
    Multiaddr,
    PeerId,
    Transport,
    StreamProtocol,
};
use std::path::Path;
use crate::{
    behaviour::P2pBehaviour as Behaviour,
    config::P2pConfig,
    error::P2pError,
    key_manager,
};

pub fn build_swarm(cfg: &P2pConfig) -> Result<(Swarm<Behaviour>, PeerId), P2pError> {
    // chave/peer id
    let key = key_manager::load_or_generate_keypair(Path::new(&cfg.keypair_path))
        .map_err(P2pError::Io)?;
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
        #[allow(deprecated)]
        cfg.set_request_timeout(std::time::Duration::from_secs(3));
    
        let protocols = std::iter::once((
            StreamProtocol::new("/atlas/tx/1"),
            ProtocolSupport::Full,
        ));
    
        RequestResponseBehaviour::new(protocols, cfg)
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
    behaviour.subscribe_core_topics()?; 

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

    Ok((swarm, peer_id))
}
