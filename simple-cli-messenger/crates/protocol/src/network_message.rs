use bytes::{BufMut, BytesMut};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Wrapper for any kind of payload which needs to be sent over the network
/// Data is packed as len + payload
#[derive(Default)]
pub struct NetworkMessage {
    payload: BytesMut,
}

impl NetworkMessage {
    pub fn new(payload: &[u8]) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u64(payload.len() as u64);
        bytes.put_slice(payload);

        NetworkMessage { payload: bytes }
    }

    pub async fn read<R: tokio::io::AsyncRead + Unpin>(read_sock: &mut R) -> io::Result<Self> {
        let mut msg_net = NetworkMessage::default();

        let msg_len = read_sock.read_u64().await?;
        let mut read = 0;
        let mut buf: [u8; 512] = [0; 512];
        while read < msg_len {
            let buf_slice = &mut buf[..(msg_len - read) as usize];
            read += read_sock.read_exact(buf_slice).await? as u64;
            msg_net.payload.extend_from_slice(buf_slice);
        }

        Ok(msg_net)
    }

    pub async fn write(
        &mut self,
        write_sock: &mut tokio::net::tcp::OwnedWriteHalf,
    ) -> anyhow::Result<()> {
        Ok(write_sock.write_all(self.payload.iter().as_slice()).await?)
    }

    pub fn get_payload(&self) -> &[u8] {
        &self.payload[..]
    }
}
