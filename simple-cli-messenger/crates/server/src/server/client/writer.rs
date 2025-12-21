use protocol::client_message::{ClientMessage, ClientMessageReceiver};

use tokio::sync::broadcast;

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

            if let ClientMessage::Data(_, receiver, _) = &msg
                && let ClientMessageReceiver::Unicast(receiver) = receiver
                && *receiver != self.username
            {
                tracing::trace!(
                    "Ignore private message destined to another user {}, we are: {}",
                    *receiver,
                    self.username
                );
                continue;
            }

            match msg.write(&mut self.tx_stream).await {
                Ok(msg) => tracing::info!(
                    "Sent message {} to client {}",
                    msg.to_json()?,
                    self.username
                ),
                Err(e) => {
                    tracing::error!("Failed to send message to client {}: {}", self.username, e);
                    break;
                }
            }
        }

        Ok(())
    }
}
