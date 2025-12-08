use clap::Parser;
use std::{io::Write, time::Duration};
use tokio::{
    io::{self, AsyncWriteExt},
    net::{
        TcpSocket,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::mpsc,
    task::JoinSet,
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
    writer: ClientWriterActorHandle,
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
        let writer = ClientWriterActorHandle::new(ClientWriter { username, tx });
        let reader = ClientReader { rx };

        Ok(Client { writer, reader })
    }

    fn into_split(self) -> (ClientReader, ClientWriterActorHandle) {
        (self.reader, self.writer)
    }

    async fn run(self) -> anyhow::Result<()> {
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
                    tracing::info!("[{}] has disconnected...", common.username);
                }
                ClientMessage::Data(common, payload) => {
                    tracing::info!("[{}]: {}", common.username, payload);
                }
                ClientMessage::Connected(common) => {
                    tracing::info!("[{}] has connected...", common.username);
                }
                _ => {}
            };
        }
    }
}

use tokio::io::{AsyncBufReadExt, BufReader};

enum ClientWriterActorMessage {
    SendData(String),
    Keepalive(),
}

struct ClientWriterActor {
    receiver: mpsc::Receiver<ClientWriterActorMessage>,
    writer: ClientWriter,
}

impl ClientWriterActor {
    fn new(receiver: mpsc::Receiver<ClientWriterActorMessage>, writer: ClientWriter) -> Self {
        Self { receiver, writer }
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        // send username of the client to the server
        let msg_cli = ClientMessage::connected(self.writer.username.clone());
        let msg_net = NetworkMessage::new(msg_cli.to_json()?.as_bytes());
        self.writer.tx.write_all(msg_net.get_payload()).await?;
        tracing::debug!("Sent: {:?}", msg_cli.to_json());

        while let Some(actor_msg) = self.receiver.recv().await {
            let msg_cli = match actor_msg {
                ClientWriterActorMessage::SendData(data) => {
                    ClientMessage::data(self.writer.username.clone(), data)
                }
                ClientWriterActorMessage::Keepalive() => {
                    ClientMessage::keepalive(self.writer.username.clone())
                }
            };

            let msg_net = NetworkMessage::new(msg_cli.to_json()?.as_bytes());
            self.writer.tx.write_all(msg_net.get_payload()).await?;

            tracing::debug!("Sent: {:?}", msg_cli.to_json());
        }

        return Ok(());
    }
}

#[derive(Clone)]
struct ClientWriterActorHandle {
    sender: mpsc::Sender<ClientWriterActorMessage>,
}

impl ClientWriterActorHandle {
    fn new(writer: ClientWriter) -> Self {
        let (sender, receiver) = mpsc::channel(100);

        let mut client_writer_actor = ClientWriterActor::new(receiver, writer);
        tokio::spawn(async move {
            client_writer_actor.run();
        });

        ClientWriterActorHandle { sender }
    }

    async fn send_message(&mut self, data: String) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::SendData(data))
            .await?)
    }

    async fn send_keepalive(&mut self) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(ClientWriterActorMessage::Keepalive())
            .await?)
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

        break Ok(input.trim_end().to_string());
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
