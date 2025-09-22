use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::network::p2p::events::AdapterEvent;
use crate::network::p2p::ports::P2pPublisher;
use crate::cluster::core::Cluster;

pub struct Maestro<P: P2pPublisher> {
    pub cluster: Arc<Cluster>,
    pub p2p: P,
    pub evt_rx: Mutex<mpsc::Receiver<AdapterEvent>>,
}

impl<P: P2pPublisher> Maestro<P> {
    pub async fn run(self: Arc<Self>) {
        loop {
            let evt = {
                let mut rx = self.evt_rx.lock().await;
                rx.recv().await
            };

            let Some(evt) = evt else { break };

            match evt {
                AdapterEvent::Proposal(bytes) => {
                    if let Err(e) = self.cluster.handle_proposal(bytes).await {
                        eprintln!("handle_proposal_bytes erro: {e}");
                        continue;
                    }
                    if let Err(e) = self.cluster.vote_proposals().await {
                        eprintln!("vote_proposals erro: {e}");
                    }
                }

                AdapterEvent::Vote(bytes) => {
                    if let Err(e) = self.cluster.handle_vote(bytes).await {
                        eprintln!("handle_vote_bytes erro: {e}");
                    }
                }

                AdapterEvent::Heartbeat{from, data} => {
                    println!("❤️ HB de {from} ({:?} bytes)", data.len());
                    tracing::debug!("❤️ HB de {from} ({:?} bytes)", data.len());
                }

                AdapterEvent::Gossip { topic, data, from } if topic == "atlas/heartbeat/v1" => {
                    tracing::info!("❤️ hb (fallback) de {from} ({} bytes)", data.len());
                }
                

                _ => {}
            }
        }
    }
}
