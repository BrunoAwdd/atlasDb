use crate::cluster::core::Cluster;

impl Cluster {
    pub fn shutdown_grpc(&mut self) {
        if let Some(tx) = self.shutdown_sender.take() {
            let _ = tx.send(()); 
            println!("🛑 Enviando sinal de shutdown para gRPC.");
        }
    }
}
