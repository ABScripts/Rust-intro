use serde::{Deserialize, Serialize};

use crate::network_message::NetworkMessage;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClientMessageCommon {
    pub username: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ClientMessageReceiver {
    Unicast(String),
    Broadcast,
}

/// In-house view on the client message
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Disconnected(ClientMessageCommon),
    Connected(ClientMessageCommon),
    Data(ClientMessageCommon, ClientMessageReceiver, String),
    KeepAlive(ClientMessageCommon),
}

// TODO: I need to figure out if there is a better way to implement this
impl ClientMessage {
    pub fn disconnected(username: String) -> Self {
        Self::Disconnected(ClientMessageCommon { username })
    }

    pub fn connected(username: String) -> Self {
        Self::Connected(ClientMessageCommon { username })
    }

    pub fn data(username: String, data: String) -> Self {
        Self::Data(
            ClientMessageCommon { username },
            ClientMessageReceiver::Broadcast,
            data,
        )
    }

    pub fn data_private(from: String, to: String, data: String) -> Self {
        Self::Data(
            ClientMessageCommon { username: from },
            ClientMessageReceiver::Unicast(to),
            data,
        )
    }

    pub fn keepalive(username: String) -> Self {
        Self::KeepAlive(ClientMessageCommon { username })
    }

    pub fn get_username(&self) -> &String {
        match self {
            Self::Disconnected(common)
            | Self::Connected(common)
            | Self::KeepAlive(common)
            | Self::Data(common, ..) => &common.username,
        }
    }

    pub fn to_json(&self) -> std::io::Result<String> {
        let msg_json = serde_json::to_string(&self)?;
        Ok(msg_json)
    }

    pub fn from_json(data: &[u8]) -> std::io::Result<Self> {
        let cli_msg = serde_json::from_slice(data)?;
        Ok(cli_msg)
    }

    pub async fn read<R: tokio::io::AsyncRead + Unpin>(read_sock: &mut R) -> anyhow::Result<Self> {
        Ok(Self::from_json(
            NetworkMessage::read(read_sock).await?.get_payload(),
        )?)
    }

    pub async fn write(
        self,
        write_sock: &mut tokio::net::tcp::OwnedWriteHalf,
    ) -> anyhow::Result<Self> {
        NetworkMessage::new(self.to_json()?.as_bytes())
            .write(write_sock)
            .await?;
        Ok(self)
    }
}
