use bytes::{BufMut, BytesMut};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;

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

    pub async fn read(read_sock: &mut tokio::net::tcp::OwnedReadHalf) -> io::Result<Self> {
        let mut msg_net = NetworkMessage::default();

        if let Ok(msg_len) = read_sock.read_u64().await {
            let mut read = 0;
            let mut buf: [u8; 512] = [0; 512];
            while read < msg_len {
                let buf_slice = &mut buf[..(msg_len - read) as usize];
                match read_sock.read_exact(buf_slice).await {
                    Ok(read_just_now) => {
                        read += read_just_now as u64;
                        msg_net.payload.extend_from_slice(buf_slice);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }

        return Ok(msg_net);
    }

    pub async fn write(
        &mut self,
        write_sock: &mut tokio::net::tcp::OwnedWriteHalf,
    ) -> anyhow::Result<()> {
        write_sock.write_all(self.payload.iter().as_slice()).await?;
        return Ok(());
    }

    pub fn get_payload(&self) -> &[u8] {
        return &self.payload[..];
    }
}
