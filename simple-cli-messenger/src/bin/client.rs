use clap::Parser;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{
        TcpSocket,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    task::JoinSet,
};

use bytes::{Buf, BufMut, BytesMut};
use std::{
    any::Any, arch::x86_64::_mm_pause, collections::HashMap, hash::Hash, io::Write, str::Bytes,
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    task::{self},
};

// This only works in lib/crate code, in src/bin I need to specify actual crate name
// use crate::client_message;

use simple_cli_messenger::client_message::ClientMessage;
use simple_cli_messenger::network_message::NetworkMessage;

#[derive(Parser)]
#[command(name = "client")]
#[command(about = "A simple CLI messenger client", long_about = None)]
struct Args {
    #[arg(long)]
    host: String,

    #[arg(short, long)]
    port: u16,
}

struct Client {
    writer: ClientWriter,
    reader: ClientReader,
}

struct ClientWriter {
    username: String, // send my messages to server side to have unified way of printing messages to the common feed?
    tx: OwnedWriteHalf,
}

struct ClientReader {
    rx: OwnedReadHalf,
}

impl Client {
    async fn connect(addr: std::net::SocketAddr, username: String) -> io::Result<Client> {
        let sock = TcpSocket::new_v4()?;
        let stream = sock.connect(addr).await?;

        let (rx, tx) = stream.into_split();
        let writer = ClientWriter { username, tx };
        let reader = ClientReader { rx };

        Ok(Client { writer, reader })
    }

    fn into_split(self) -> (ClientReader, ClientWriter) {
        (self.reader, self.writer)
    }

    async fn run(self) -> anyhow::Result<()> {
        let (rx, tx) = self.into_split();
        let mut join_set = JoinSet::new();

        join_set.spawn(tx.chat());
        join_set.spawn(rx.read_incoming());

        join_set.join_all().await;

        Ok(())
    }
}

impl ClientReader {
    async fn read_incoming(mut self) -> anyhow::Result<()> {
        loop {
            let msg_net = NetworkMessage::read(&mut self.rx).await?;
            tracing::debug!(
                "Received message; serialized view: |{:?}|",
                msg_net.get_payload()
            );

            let msg_cli = match ClientMessage::from_json(msg_net.get_payload()) {
                Err(e) => {
                    tracing::error!(
                        "Failed to construct ClientMessage from serizealized view: |{:?}|, error: {e}",
                        msg_net.get_payload()
                    );
                    continue;
                }
                Ok(msg_cli) => msg_cli,
            };

            match msg_cli {
                ClientMessage::Disconnected(common) => {
                    tracing::info!("Client {} has disconnected...", common.id);
                }
                ClientMessage::Data(common, payload) => {
                    tracing::info!("From {}: {}", common.id, payload);
                }
                ClientMessage::Connected(common) => {
                    tracing::info!("Client {} has connected...", common.id);
                }
            };
        }
    }
}

use tokio::io::{AsyncBufReadExt, BufReader};

impl ClientWriter {
    async fn chat(mut self) -> anyhow::Result<()> {
        loop {
            print!("Type: ");
            std::io::stdout().flush()?; // to actually see the above printed line (avoid buffering)

            let mut reader = BufReader::new(tokio::io::stdin());
            let mut message = Vec::new();
            match reader.read_until(b'\n', &mut message).await {
                Ok(_) => {
                    let msg_cli = ClientMessage::data(0, String::from_utf8(message)?);
                    let msg_net = NetworkMessage::new(msg_cli.to_json()?.as_bytes());

                    self.tx.write_all(msg_net.get_payload()).await?;
                    tracing::debug!("Sent: {:?}", msg_cli.to_json());
                }
                Err(e) => {
                    tracing::error!("Failed to get input from user: {}", e);
                    continue;
                }
            }
        }
    }
}

fn input_username() -> std::io::Result<String> {
    loop {
        print!("Enter username to join the chat: ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().is_empty() {
            tracing::warn!("Username cannot be empty. Try again.");
            continue;
        }

        break Ok(input.to_string());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let username = match input_username() {
        Err(e) => {
            tracing::error!("Failed to read username.");
            return Err(e.into());
        }
        Ok(username) => username,
    };

    let addr = format!("{}:{}", args.host, args.port).parse()?;
    let client = Client::connect(addr, username).await?;
    client.run().await?;

    Ok(())
}
