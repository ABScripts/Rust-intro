use super::ServerState;
use protocol::client_message::ClientMessage;
use std::sync::Arc;

use tokio::sync::mpsc;
pub struct ClientReader {
    username: String,
    rx_stream: tokio::net::tcp::OwnedReadHalf,
    tx_to_message_distributor: mpsc::Sender<ClientMessage>,
    server_state: Arc<ServerState>,
    to_writer: tokio::sync::mpsc::Sender<ClientMessage>,
}

impl ClientReader {
    pub fn new(
        username: String,
        rx_stream: tokio::net::tcp::OwnedReadHalf,
        tx_to_message_distributor: mpsc::Sender<ClientMessage>,
        server_state: Arc<ServerState>,
        to_writer: tokio::sync::mpsc::Sender<ClientMessage>,
    ) -> Self {
        Self {
            username,
            rx_stream,
            tx_to_message_distributor,
            server_state,
            to_writer,
        }
    }

    pub async fn handle_incoming(&mut self) -> anyhow::Result<()> {
        tracing::info!("Started getting messages");

        loop {
            match ClientMessage::read(&mut self.rx_stream).await? {
                ClientMessage::GetUsers() => {
                    let users = self
                        .server_state
                        .connected_clients
                        .read()
                        .await
                        .keys()
                        .cloned()
                        .collect::<Vec<String>>();

                    self.to_writer
                        .send(ClientMessage::Users(serde_json::to_string(&users).unwrap()))
                        .await?;
                }
                ClientMessage::Kick(common, kicked_username) => {
                    // remove user object
                    // next arc clone is held inside client specific task
                    // its connection will be dropped as soon as it receives kick message
                    if common.username != kicked_username {
                        self.server_state
                            .connected_clients
                            .write()
                            .await
                            .remove(&kicked_username);
                        self.tx_to_message_distributor
                            .send(ClientMessage::Kick(common, kicked_username))
                            .await?
                    }
                }
                msg => {
                    tracing::info!("Received message from client {}: {:?}", self.username, msg);
                    self.tx_to_message_distributor.send(msg).await?;
                }
            }
        }
    }
}
