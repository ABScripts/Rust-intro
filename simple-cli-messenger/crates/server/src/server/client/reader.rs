use protocol::client_message::ClientMessage;
use protocol::network_message::NetworkMessage;

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
            match NetworkMessage::read(&mut self.rx_stream).await {
                Ok(msg_net) => {
                    tracing::info!("Parsing message");
                    let msg_cli = ClientMessage::from_json(msg_net.get_payload())?;

                    tracing::info!(
                        "Received message from client {}: {:?}",
                        self.username,
                        msg_cli
                    );

                    if self.tx_to_message_distributor.send(msg_cli).await.is_err() {
                        tracing::error!(
                            "Failed to redistribute message from client {}",
                            self.username
                        );
                        break;
                    }
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
