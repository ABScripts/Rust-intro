mod reader;
mod writer;

use crate::client::reader::ClientReader;
use crate::client::writer::ClientWriter;
use crate::client::writer::ClientWriterActorHandle;

use std::{io::Write, time::Duration};
use tokio::io::BufReader;
use tokio::{
    io::{self, AsyncBufReadExt},
    net::TcpSocket,
    task::JoinSet,
};

pub struct Client {
    writer: ClientWriterActorHandle,
    reader: ClientReader,
}

impl Client {
    pub async fn connect(addr: std::net::SocketAddr, username: String) -> io::Result<Client> {
        let sock = TcpSocket::new_v4()?;
        let stream = sock.connect(addr).await?;

        let (rx, tx) = stream.into_split();
        let writer = ClientWriterActorHandle::new(ClientWriter::new(username, tx));
        let reader = ClientReader::new(rx);

        Ok(Client { writer, reader })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (rx, tx) = self.into_split();

        let mut join_set = JoinSet::new();
        join_set.spawn({
            let mut client_writer_actor_handle = tx.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    client_writer_actor_handle.send_keepalive().await;
                }
            }
        });
        join_set.spawn({
            let mut client_writer_actor_handle = tx.clone();
            async move {
                loop {
                    print!("Type: ");
                    std::io::stdout().flush()?; // to actually see the above printed line (avoid buffering)

                    let mut reader = BufReader::new(tokio::io::stdin());
                    let mut message = Vec::new();
                    match reader.read_until(b'\n', &mut message).await {
                        Ok(_) => {
                            client_writer_actor_handle
                                .send_message(String::from_utf8(message)?)
                                .await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to get input from user: {}", e);
                            continue;
                        }
                    }
                }
            }
        });
        join_set.spawn(rx.read_incoming());

        join_set.join_all().await;

        Ok(())
    }

    fn into_split(self) -> (ClientReader, ClientWriterActorHandle) {
        (self.reader, self.writer)
    }
}
