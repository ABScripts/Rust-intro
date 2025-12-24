use crate::client::reader::ClientReader;
use crate::client::writer::ClientWriter;
use crate::client::writer::ClientWriterActorHandle;
use protocol::client_message::ClientMessage;

use std::{io::Write, time::Duration};
use tokio::{
    io::{self},
    net::TcpSocket,
    task::JoinSet,
};

pub struct ClientAdmin {
    writer: ClientWriterActorHandle,
    reader: ClientReader,
}

impl ClientAdmin {
    pub async fn connect(addr: std::net::SocketAddr, username: String) -> io::Result<Self> {
        let sock = TcpSocket::new_v4()?;
        let stream = sock.connect(addr).await?;

        let (rx, tx) = stream.into_split();
        let writer = ClientWriterActorHandle::new(ClientWriter::new(username, tx));
        let reader = ClientReader::new(rx);

        Ok(Self { writer, reader })
    }

    pub async fn send_private(self: &mut Self, to: String, msg: String) {
        let _ = self.writer.send_private_message(to, msg).await;
    }

    pub async fn kick_user(self: &mut Self, who: String) {
        todo!();
    }

    pub async fn get_users(self: &mut Self, who: String) {
        todo!();
    }

    // nasty workaround...
    // to be able to send messages to the server
    // as client will be moved
    pub fn get_writer(&self) -> ClientWriterActorHandle {
        self.writer.clone()
    }

    pub async fn run(
        self,
        tx_to_external: tokio::sync::mpsc::Sender<ClientMessage>,
    ) -> anyhow::Result<()> {
        let mut join_set = JoinSet::new();
        join_set.spawn(Self::send_keepalives(self.writer.clone()));
        join_set.spawn(self.reader.forward_messages(tx_to_external));

        Ok(join_set.join_next().await.unwrap()??)
    }

    // This seems to be the cleanest way of defining these tasks, based on the reqs:
    // 1) Automatic return type deduction instead of explicitly specifying returned type:
    //    #[allow(unreachable_code)]
    //    Ok::<(), anyhow::Error>(())
    // 2) Ability to name that logical piece
    async fn send_keepalives(mut writer: ClientWriterActorHandle) -> anyhow::Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            writer.send_keepalive().await?;
        }
    }
}
