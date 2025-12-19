mod actor;
mod message;

use crate::client::writer::actor::ClientWriterActor;
use crate::client::writer::message::ClientWriterActorMessage;

use tokio::{net::tcp::OwnedWriteHalf, sync::mpsc};

pub struct ClientWriter {
    username: String, // send my messages to server side to have unified way of printing messages to the common feed?
    tx: OwnedWriteHalf,
}

impl ClientWriter {
    pub fn new(username: String, tx: OwnedWriteHalf) -> Self {
        Self { username, tx }
    }
}

#[derive(Clone)]
pub struct ClientWriterActorHandle {
    sender: mpsc::Sender<ClientWriterActorMessage>,
}

impl ClientWriterActorHandle {
    pub fn new(writer: ClientWriter) -> Self {
        let (sender, receiver) = mpsc::channel(100);

        let mut client_writer_actor = ClientWriterActor::new(receiver, writer);
        tokio::spawn(async move {
            client_writer_actor.run().await;
        });

        ClientWriterActorHandle { sender }
    }

    pub async fn send_message(&mut self, data: String) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::SendData(data))
            .await?)
    }

    pub async fn send_keepalive(&mut self) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::Keepalive())
            .await?)
    }
}
