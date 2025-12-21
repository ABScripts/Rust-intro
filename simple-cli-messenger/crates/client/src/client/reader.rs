use protocol::client_message::ClientMessage;

use tokio::net::tcp::OwnedReadHalf;

pub struct ClientReader {
    rx: OwnedReadHalf,
}

impl ClientReader {
    pub fn new(rx: OwnedReadHalf) -> Self {
        Self { rx }
    }

    pub async fn read_incoming_messages(mut self) -> anyhow::Result<()> {
        loop {
            match ClientMessage::read(&mut self.rx).await? {
                ClientMessage::Disconnected(common) => {
                    tracing::info!("[{}] has disconnected...", common.username);
                }
                ClientMessage::Data(common, payload) => {
                    tracing::info!("[{}]: {}", common.username, payload);
                }
                ClientMessage::Connected(common) => {
                    tracing::info!("[{}] has connected...", common.username);
                }
                _ => {}
            };
        }
    }
}
