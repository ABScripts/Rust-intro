use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{tcp:: {OwnedWriteHalf, OwnedReadHalf}, TcpSocket},
    task::JoinSet,
};

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
            let read = self.rx.read_f32().await?;
            println!("Received message: {read}");
        }
    }
}

impl ClientWriter {
    async fn chat(mut self) -> anyhow::Result<()> {
        loop {
            let mut input = String::new();
            println!("Enter a number to send: ");
            std::io::stdin().read_line(&mut input)?;

            println!("User input: {input}");

            match input.trim().parse::<f32>() {
                Ok(num) => self.tx.write_f32(num).await?,
                Err(_) => {
                    eprintln!("Invalid number, try again.");
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
    // use anyhow as I have two different methods returning Result which is defined in different modules
    let username = loop {
        match input_username() {
            Ok(username) => break username,
            Err(e) => {
                eprintln!("Failed to read username: {e}. Try again.");
                continue;
            }
        }
    };

    let addr = "127.0.0.1:3456".parse()?;
    let client = Client::connect(addr, username).await?;
    client.run().await?;

    Ok(())
}
