use protocol::client_message::ClientMessage;

use tokio::sync::mpsc;

pub struct ClientReader {
    username: String,
    rx_stream: tokio::net::tcp::OwnedReadHalf,
    tx_to_message_distributor: mpsc::Sender<ClientMessage>,
}

impl ClientReader {
    pub fn new(
        username: String,
        rx_stream: tokio::net::tcp::OwnedReadHalf,
        tx_to_message_distributor: mpsc::Sender<ClientMessage>,
    ) -> Self {
        Self {
            username,
            rx_stream,
            tx_to_message_distributor,
        }
    }

    pub async fn handle_incoming(&mut self) -> anyhow::Result<()> {
        tracing::info!("Started getting messages");

        loop {
            match ClientMessage::read(&mut self.rx_stream).await {
                Ok(msg) => {
                    tracing::info!("Received message from client {}: {:?}", self.username, msg);
                    self.tx_to_message_distributor.send(msg).await?;
                }
                Err(e) => {
                    self.tx_to_message_distributor
                        .send(ClientMessage::disconnected(self.username.clone()))
                        .await?;

                    tracing::info!("Client {} disconnected: {}", self.username, e);

                    break;
                }
            }
        }

        Ok(())
    }
}
