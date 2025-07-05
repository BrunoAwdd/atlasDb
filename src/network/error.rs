use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Erro ao enviar a mensagem: {0}")]
    Send(String),

    #[error("Erro ao receber a mensagem: {0}")]
    Receive(String),

    #[error("Handler de mensagem não configurado")]
    HandlerNotSet,

    #[error("Erro de serialização: {0}")]
    Serialization(String),

    #[error("Erro de conexão: {0}")]
    ConnectionError(String),

    #[error("Erro desconhecido")]
    Unknown,
}