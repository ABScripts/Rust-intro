use protocol::client_message::ClientMessage;
use protocol::network_message::NetworkMessage;

use tokio::{io::AsyncWriteExt, sync::broadcast};

pub struct ClientWriter {
    username: String,
    tx_stream: tokio::net::tcp::OwnedWriteHalf,
    rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
}

impl ClientWriter {
    pub fn new(
        username: String,
        tx_stream: tokio::net::tcp::OwnedWriteHalf,
        rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
    ) -> Self {
        Self {
            username,
            tx_stream,
            rx_from_message_distributor,
        }
    }

    pub async fn handle_outgoing(&mut self) -> anyhow::Result<()> {
        while let Ok(msg) = self.rx_from_message_distributor.recv().await {
            if *msg.get_username() == self.username {
                tracing::trace!(
                    "Ignore message destined to {}, we are: {}",
                    msg.get_username(),
                    self.username
                );
                continue;
            }

            let msg_json = msg.to_json()?;
            tracing::debug!("Sending {msg_json}");

            let msg_net = NetworkMessage::new(msg_json.as_bytes());
            tracing::debug!("Serialized view: |{:?}|", msg_net.get_payload());

            match self.tx_stream.write(&msg_net.get_payload()).await {
                Ok(_) => tracing::info!("Sent message {} to client {}", msg_json, self.username),
                Err(e) => {
                    tracing::error!("Failed to send message to client {}: {}", self.username, e);
                    break;
                }
            }
        }

        Ok(())
    }
}
