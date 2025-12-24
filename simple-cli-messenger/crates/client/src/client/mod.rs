mod reader;
// nasty workaround
pub mod client_admin;
pub mod writer;

use crate::client::reader::ClientReader;
use crate::client::writer::ClientWriter;
use crate::client::writer::ClientWriterActorHandle;


use std::{io::Write, time::Duration};
use tokio::{
    io::{self},
    net::TcpSocket,
    task::JoinSet,
};

pub struct Client {
    writer: ClientWriterActorHandle,
    reader: ClientReader,
}

impl Client {
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

    pub async fn run(self) -> anyhow::Result<()> {
        let mut join_set = JoinSet::new();
        join_set.spawn(Self::send_keepalives(self.writer.clone()));
        join_set.spawn(Self::handle_input(self.writer.clone()));
        join_set.spawn(self.reader.read_incoming_messages());

        // 1) It should be fine to unwrap here as "None" will be returned only in case
        // the join set is empty and here it is clearly not?
        // 2) First "?"  - on error coming from the joining operation itself
        //    Second "?" - on error reported from the task
        Ok(join_set.join_next().await.unwrap()??)
    }

    // This seems to be the cleanest way of defining these tasks, based on the reqs:
    // 1) Automatic return type deduction instead of explicitly specifying returned type:
    //    #[allow(unreachable_code)]
    //    Ok::<(), anyhow::Error>(())
    // 2) Ability to name that logical piece
    async fn send_keepalives(mut writer: ClientWriterActorHandle) -> anyhow::Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(120)).await;
            writer.send_keepalive().await?;
        }
    }

    async fn handle_input(mut writer: ClientWriterActorHandle) -> anyhow::Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // With tokio's stdin and buf reader it would stuck, quoting their docs:
        //  "For technical reasons, stdin is implemented by using an
        //   ordinary blocking read on a separate thread, and it is impossible
        //   to cancel that read. This can make shutdown of the runtime hang
        //   until the user presses enter."
        std::thread::spawn(move || -> anyhow::Result<()> {
            loop {
                print!("Type: ");
                std::io::stdout().flush()?;

                let mut buffer = String::new();
                std::io::stdin().read_line(&mut buffer)?;
                tx.blocking_send(buffer)?;
            }
        });

        // With user input handled in a dedicated thread, tokio's runner wouldn't stuck
        // This task would be aborted, the program will shut automatically killing the above thread
        while let Some(message) = rx.recv().await {
            writer.send_message(message).await?;
        }

        Ok(())
    }
}
