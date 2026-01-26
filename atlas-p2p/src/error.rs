use thiserror::Error;

#[derive(Debug, Error)]
pub enum P2pError {
    #[error("falha ao subscrever tópico gossipsub: {0}")]
    Gossipsub(#[from] libp2p::gossipsub::SubscriptionError),

    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("multiaddr inválido: {0}")]
    Multiaddr(#[from] libp2p::multiaddr::Error),

    #[error("erro de transporte: {0}")]
    Transport(#[from] libp2p::TransportError<std::io::Error>),

    // (opcionais, mas provavelmente úteis em seguida:)
    #[error("erro ao discar peer/endereço: {0}")]
    Dial(#[from] libp2p::swarm::DialError),

    #[error("erro no noise: {0}")]
    Noise(#[from] libp2p::noise::Error),

    #[error("gossipsub init error: {0}")]
    GossipsubInit(&'static str),

}
