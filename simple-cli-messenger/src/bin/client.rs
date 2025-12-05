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
use std::{arch::x86_64::_mm_pause, collections::HashMap, hash::Hash, str::Bytes, sync::Arc};
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
            match NetworkMessage::read(&mut self.rx).await {
                Ok(msg_net) => {
                    eprintln!("Received serialized view: |{:?}|", msg_net.get_payload());

                    let msg_json = ClientMessage::from_json(msg_net.get_payload());
                    eprintln!("Received JSON: {:?}", msg_json);
                }
                Err(e) => {
                    eprintln!("Failed to receive message: {e}");
                    return Err(e.into());
                }
            }
        }
    }
}

use tokio::io::{AsyncBufReadExt, BufReader};

impl ClientWriter {
    async fn chat(mut self) -> anyhow::Result<()> {
        loop {
            eprint!("Type: "); // otherwise the output is not flushed to stdin in time; just quickest way
            let mut reader = BufReader::new(tokio::io::stdin());
            let mut message = Vec::new();
            match reader.read_until(b'\n', &mut message).await {
                Ok(_) => {
                    let msg_cli = ClientMessage::data(0, String::from_utf8(message)?);
                    let msg_net = NetworkMessage::new(msg_cli.to_json()?.as_bytes());

                    self.tx.write_all(msg_net.get_payload()).await?;
                    println!("Sent: {:?}", msg_cli.to_json());
                }
                Err(e) => {
                    println!("Error: {}", e);
                    continue;
                }
            }
        }
    }
}

fn input_username() -> std::io::Result<String> {
    loop {
        let mut input = String::new();
        println!("Enter username to join the chat: ");
        std::io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            eprintln!("Username cannot be empty");
            continue;
        }

        break Ok(input.to_string());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let username = loop {
        match input_username() {
            Ok(username) => break username,
            Err(e) => {
                eprintln!("Failed to read username: {e}. Try again.");
                continue;
            }
        }
    };

    let addr = format!("{}:{}", args.host, args.port).parse()?;
    let client = Client::connect(addr, username).await?;
    client.run().await?;

    Ok(())
}
