use async_trait::async_trait;
use futures::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use libp2p::{request_response as rr, StreamProtocol}; // <- raiz, não swarm
use std::io;

use crate::network::p2p::protocol::{TxRequest, TxBundle};

#[derive(Clone, Default)]
pub struct TxCodec;

#[async_trait]
impl rr::Codec for TxCodec {
    type Protocol = StreamProtocol;
    type Request  = TxRequest;
    type Response = TxBundle;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T)
        -> io::Result<Self::Request>
    where T: AsyncRead + Unpin + Send
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        bincode::deserialize(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T)
        -> io::Result<Self::Response>
    where T: AsyncRead + Unpin + Send
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        bincode::deserialize(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request)
        -> io::Result<()>
    where T: AsyncWrite + Unpin + Send
    {
        let bytes = bincode::serialize(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = bytes.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&bytes).await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response)
        -> io::Result<()>
    where T: AsyncWrite + Unpin + Send
    {
        let bytes = bincode::serialize(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = bytes.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&bytes).await?;
        Ok(())
    }
}
