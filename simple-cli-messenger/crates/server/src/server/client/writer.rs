use protocol::client_message::{ClientMessage, ClientMessageReceiver};

use tokio::sync::broadcast;

pub struct ClientWriter {
    username: String,
    tx_stream: tokio::net::tcp::OwnedWriteHalf,
    rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
    from_reader: tokio::sync::mpsc::Receiver<ClientMessage>,
}

enum MessageHandlingResult {
    Message(ClientMessage),
    IgnoreForeignMessage,
    ReceivedKick,
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
    ) -> anyhow::Result<MessageHandlingResult> {
        if let Some(from_username) = msg.get_username()
            && *from_username == self.username
        {
            tracing::trace!(
                "Ignore message destined to {}, we are: {}",
                *from_username,
                self.username
            );
            return Ok(MessageHandlingResult::IgnoreForeignMessage);
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
            return Ok(MessageHandlingResult::IgnoreForeignMessage);
        }

        if let ClientMessage::Kick(common, who) = &msg
            && *who == self.username
        {
            tracing::warn!("{} kicked us", common.username);
            return Ok(MessageHandlingResult::ReceivedKick);
        }

        Ok(MessageHandlingResult::Message(
            msg.write(&mut self.tx_stream).await?,
        ))
    }

    async fn handle_direct_message(
        &mut self,
        msg: ClientMessage,
    ) -> anyhow::Result<MessageHandlingResult> {
        Ok(MessageHandlingResult::Message(
            msg.write(&mut self.tx_stream).await?,
        ))
    }

    pub async fn handle_outgoing(&mut self) -> anyhow::Result<()> {
        loop {
            let res = tokio::select! {
                Ok(msg) = self.rx_from_message_distributor.recv() => {
                    self.handle_broadcast_message(msg).await
                },
                Some(msg) = self.from_reader.recv() => {
                    self.handle_direct_message(msg).await
                },
            };

            let res = match res {
                Err(e) => {
                    tracing::error!("Failed to send message to client {}: {}", self.username, e);
                    return Err(e);
                }
                Ok(res) => res,
            };

            match res {
                MessageHandlingResult::IgnoreForeignMessage => {}
                MessageHandlingResult::ReceivedKick => {
                    // wrap up this task && make the client shutdown
                    return Ok(());
                }
                MessageHandlingResult::Message(msg) => {
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
