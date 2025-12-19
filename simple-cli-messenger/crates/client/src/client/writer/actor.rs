use crate::client::writer::ClientWriter;
use crate::client::writer::message::ClientWriterActorMessage;
use protocol::client_message::ClientMessage;
use protocol::network_message::NetworkMessage;

use tokio::{io::AsyncWriteExt, sync::mpsc};

pub struct ClientWriterActor {
    receiver: mpsc::Receiver<ClientWriterActorMessage>,
    writer: ClientWriter,
}

impl ClientWriterActor {
    pub fn new(receiver: mpsc::Receiver<ClientWriterActorMessage>, writer: ClientWriter) -> Self {
        Self { receiver, writer }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // send username of the client to the server
        let msg_cli = ClientMessage::connected(self.writer.username.clone());
        let msg_net = NetworkMessage::new(msg_cli.to_json()?.as_bytes());
        self.writer.tx.write_all(msg_net.get_payload()).await?;
        tracing::debug!("Sent: {:?}", msg_cli.to_json());

        while let Some(actor_msg) = self.receiver.recv().await {
            let msg_cli = match actor_msg {
                ClientWriterActorMessage::SendData(data) => {
                    ClientMessage::data(self.writer.username.clone(), data)
                }
                ClientWriterActorMessage::Keepalive() => {
                    ClientMessage::keepalive(self.writer.username.clone())
                }
            };

            let msg_net = NetworkMessage::new(msg_cli.to_json()?.as_bytes());
            self.writer.tx.write_all(msg_net.get_payload()).await?;

            tracing::debug!("Sent: {:?}", msg_cli.to_json());
        }

        return Ok(());
    }
}
