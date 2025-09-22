use async_trait::async_trait;
use futures::io::{AsyncRead, AsyncWrite};
use libp2p::{request_response as rr, StreamProtocol}; // <- raiz, não swarm
use std::io;

use super::protocol::{TxRequest, TxBundle};

#[derive(Clone, Default)]
pub struct TxCodec;

#[async_trait]
impl rr::Codec for TxCodec {
    type Protocol = StreamProtocol;
    type Request  = TxRequest;
    type Response = TxBundle;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, _io: &mut T)
        -> io::Result<Self::Request>
    where T: AsyncRead + Unpin + Send
    {
        unimplemented!()
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, _io: &mut T)
        -> io::Result<Self::Response>
    where T: AsyncRead + Unpin + Send
    {
        unimplemented!()
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, _io: &mut T, _req: Self::Request)
        -> io::Result<()>
    where T: AsyncWrite + Unpin + Send
    {
        unimplemented!()
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, _io: &mut T, _res: Self::Response)
        -> io::Result<()>
    where T: AsyncWrite + Unpin + Send
    {
        unimplemented!()
    }
}
