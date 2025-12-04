use crate::cluster::core::Cluster;

impl Cluster {
    pub(super) async fn shutdown_grpc(&self) {
        if let Some(sender) = self.shutdown_sender.lock().await.take() {
            let _ = sender.send(());
            println!("🔴 gRPC shutdown enviado com sucesso");
        } else {
            println!("⚠️ shutdown_sender já foi usado ou não estava configurado");
        }
    }
}