use crate::client::writer::ClientWriter;
use crate::client::writer::message::ClientWriterActorMessage;
use protocol::client_message::ClientMessage;
use protocol::client_message::ClientMessageReceiver;

use tokio::sync::mpsc;

pub struct ClientWriterActor {
    receiver: mpsc::Receiver<ClientWriterActorMessage>,
    writer: ClientWriter,
}

impl ClientWriterActor {
    pub fn new(receiver: mpsc::Receiver<ClientWriterActorMessage>, writer: ClientWriter) -> Self {
        Self { receiver, writer }
    }

    async fn send_username_to_server(&mut self) -> anyhow::Result<()> {
        let msg_cli = ClientMessage::connected(self.writer.username.clone())
            .write(&mut self.writer.tx)
            .await?;
        tracing::debug!("Sent: {:?}", msg_cli.to_json());
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.send_username_to_server().await?;

        while let Some(actor_msg) = self.receiver.recv().await {
            let msg_cli = match actor_msg {
                ClientWriterActorMessage::SendData(data, to) => match to {
                    ClientMessageReceiver::Broadcast => {
                        ClientMessage::data(self.writer.username.clone(), data)
                    }
                    ClientMessageReceiver::Unicast(to) => {
                        ClientMessage::data_private(self.writer.username.clone(), to, data)
                    }
                },
                ClientWriterActorMessage::Keepalive() => {
                    ClientMessage::keepalive(self.writer.username.clone())
                }
                ClientWriterActorMessage::GetUsers() => ClientMessage::GetUsers(),
                ClientWriterActorMessage::KickUser(who) => {
                    ClientMessage::kick_user(self.writer.username.clone(), who)
                }
            }
            .write(&mut self.writer.tx)
            .await?;

            tracing::debug!("Sent: {:?}", msg_cli.to_json());
        }

        Ok(())
    }
}
