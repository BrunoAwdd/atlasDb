// Este arquivo define o módulo RPC e importa o código gerado pelo Prost/Tonic.

// pub mod server;
pub mod client;

pub mod atlas {
    tonic::include_proto!("atlas");
}
