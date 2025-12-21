mod actor;
mod message;

use crate::client::writer::actor::ClientWriterActor;
use crate::client::writer::message::ClientWriterActorMessage;
use protocol::client_message::ClientMessageReceiver;

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

        // This task would automatically end as soon as producers die
        // Until there is someone who needs us, we should be running
        tokio::spawn(async {
            let _ = ClientWriterActor::new(receiver, writer).run().await;
        });

        Self { sender }
    }

    pub async fn send_message(&mut self, data: String) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::SendData(
                data,
                ClientMessageReceiver::Broadcast,
            ))
            .await?)
    }

    pub async fn send_private_message(&mut self, to: String, data: String) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::SendData(
                data,
                ClientMessageReceiver::Unicast(to),
            ))
            .await?)
    }

    pub async fn send_keepalive(&mut self) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::Keepalive())
            .await?)
    }
}
