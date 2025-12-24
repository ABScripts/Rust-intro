
use protocol::client_message::{ClientMessage, ClientMessageReceiver};

use tokio::net::tcp::OwnedReadHalf;

pub struct ClientReader {
    rx: OwnedReadHalf,
}

impl ClientReader {
    pub fn new(rx: OwnedReadHalf) -> Self {
        Self { rx }
    }

    // Specifically added to support message forwarding need for the admin panel user
    pub async fn forward_messages(
        mut self,
        tx_to_external: tokio::sync::mpsc::Sender<ClientMessage>,
    ) -> anyhow::Result<()> {
        loop {
            let msg = ClientMessage::read(&mut self.rx).await?;
            tx_to_external.send(msg).await?;
        }
    }

    pub async fn read_incoming_messages(mut self) -> anyhow::Result<()> {
        loop {
            match ClientMessage::read(&mut self.rx).await? {
                ClientMessage::Disconnected(common) => {
                    tracing::info!("[{}] has disconnected...", common.username);
                }
                ClientMessage::Data(common, dest, payload) => {
                    let mut info = format!("[{}]", common.username);
                    if let ClientMessageReceiver::Unicast(_) = dest {
                        info += " 🔒";
                    }
                    info += &format!("{}", payload);

                    tracing::info!("{}", info);
                }
                ClientMessage::Connected(common) => {
                    tracing::info!("[{}] has connected...", common.username);
                }
                _ => {}
            };
        }
    }
}
