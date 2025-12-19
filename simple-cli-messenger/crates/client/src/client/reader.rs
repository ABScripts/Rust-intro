use protocol::client_message::ClientMessage;
use protocol::network_message::NetworkMessage;

use tokio::net::tcp::OwnedReadHalf;

pub struct ClientReader {
    rx: OwnedReadHalf,
}

impl ClientReader {
    pub fn new(rx: OwnedReadHalf) -> Self {
        Self { rx }
    }

    pub async fn read_incoming(mut self) -> anyhow::Result<()> {
        loop {
            let msg_net = NetworkMessage::read(&mut self.rx).await?;
            tracing::debug!(
                "Received message; serialized view: |{:?}|",
                msg_net.get_payload()
            );

            let msg_cli = match ClientMessage::from_json(msg_net.get_payload()) {
                Err(e) => {
                    tracing::error!(
                        "Failed to construct ClientMessage from serizealized view: |{:?}|, error: {e}",
                        msg_net.get_payload()
                    );
                    continue;
                }
                Ok(msg_cli) => msg_cli,
            };

            match msg_cli {
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
