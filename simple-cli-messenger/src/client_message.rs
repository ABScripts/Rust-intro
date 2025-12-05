use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ClientMessageCommon {
    id: u8,
}

/// In-house view on the client message
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Disconnected(ClientMessageCommon),
    Connected(ClientMessageCommon),
    Data(ClientMessageCommon, String),
}

// TODO: I need to figure out if there is a better way to implement this
impl ClientMessage {
    pub fn disconnected(id: u8) -> Self {
        return ClientMessage::Disconnected(ClientMessageCommon { id });
    }

    pub fn connected(id: u8) -> Self {
        return ClientMessage::Connected(ClientMessageCommon { id });
    }

    pub fn data(id: u8, data: String) -> Self {
        return ClientMessage::Data(ClientMessageCommon { id }, data);
    }

    pub fn get_id(&self) -> u8 {
        match self {
            ClientMessage::Disconnected(common)
            | ClientMessage::Connected(common)
            | ClientMessage::Data(common, ..) => common.id,
        }
    }

    pub fn set_id(&mut self, id: u8) {
        match self {
            ClientMessage::Disconnected(common)
            | ClientMessage::Connected(common)
            | ClientMessage::Data(common, ..) => common.id = id,
        }
    }

    pub fn to_json(&self) -> std::io::Result<String> {
        let msg_json = serde_json::to_string(&self)?;
        return Ok(msg_json);
    }

    pub fn from_json(data: &[u8]) -> std::io::Result<ClientMessage> {
        let cli_msg = serde_json::from_slice(data)?;
        return Ok(cli_msg);
    }
}
