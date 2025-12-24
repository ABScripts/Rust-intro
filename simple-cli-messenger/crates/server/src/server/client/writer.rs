use protocol::client_message::{ClientMessage, ClientMessageReceiver};

use tokio::sync::broadcast;

pub struct ClientWriter {
    username: String,
    tx_stream: tokio::net::tcp::OwnedWriteHalf,
    rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
    from_reader: tokio::sync::mpsc::Receiver<ClientMessage>,
}

impl ClientWriter {
    pub fn new(
        username: String,
        tx_stream: tokio::net::tcp::OwnedWriteHalf,
        rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
        from_reader: tokio::sync::mpsc::Receiver<ClientMessage>,
    ) -> Self {
        Self {
            username,
            tx_stream,
            rx_from_message_distributor,
            from_reader,
        }
    }

    async fn handle_broadcast_message(
        &mut self,
        msg: ClientMessage,
    ) -> anyhow::Result<Option<ClientMessage>> {
        if let Some(from_username) = msg.get_username()
            && *from_username == self.username
        {
            tracing::trace!(
                "Ignore message destined to {}, we are: {}",
                *from_username,
                self.username
            );
            return Ok(None);
        }

        if let ClientMessage::Data(_, to_username, _) = &msg
            && let ClientMessageReceiver::Unicast(to_username) = to_username
            && *to_username != self.username
        {
            tracing::trace!(
                "Ignore private message destined to {}, we are: {}",
                *to_username,
                self.username
            );
            return Ok(None);
        }

        let msg = msg.write(&mut self.tx_stream).await?;
        tracing::info!(
            "Sent message {} to client {}",
            msg.to_json()?,
            self.username
        );

        Ok(Some(msg))
    }

    pub async fn handle_direct_message(
        &mut self,
        msg: ClientMessage,
    ) -> anyhow::Result<ClientMessage> {
        Ok(msg.write(&mut self.tx_stream).await?)
    }

    pub async fn handle_outgoing(&mut self) -> anyhow::Result<()> {
        loop {
            let res = tokio::select! {
                Ok(msg) = self.rx_from_message_distributor.recv() => {
                    match self.handle_broadcast_message(msg).await {
                        Ok(None) => continue,     // message wasn't destined to us
                        Ok(Some(msg)) => Ok(msg),
                        Err(e) => Err(e)
                    }
                },
                Some(msg) = self.from_reader.recv() => {
                    self.handle_direct_message(msg).await
                },
            };

            match res {
                Err(e) => {
                    tracing::error!("Failed to send message to client {}: {}", self.username, e);
                    return Err(e);
                }
                Ok(msg) => {
                    tracing::info!(
                        "Sent message {} to client {}",
                        msg.to_json()?,
                        self.username
                    );
                }
            }
        }
    }
}
