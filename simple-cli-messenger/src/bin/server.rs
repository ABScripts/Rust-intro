use bytes::{Buf, BufMut, BytesMut};
use std::{arch::x86_64::_mm_pause, collections::HashMap, hash::Hash, str::Bytes, sync::Arc};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    task::{self, JoinSet},
};

use ::tokio::sync::Mutex;

use simple_cli_messenger::client_message::ClientMessage;
use simple_cli_messenger::network_message::NetworkMessage;

struct Server {
    listener: TcpListener,
    tx_to_message_distributor: mpsc::Sender<ClientMessage>,
    tx_to_clients: broadcast::Sender<ClientMessage>,
    connected_clients: HashMap<u8, Arc<Client>>,
}

struct Client {
    id: u8,
    reader: Arc<Mutex<ClientReader>>,
    writer: Arc<Mutex<ClientWriter>>,
}

struct ClientReader {
    id: u8,
    rx_stream: tokio::net::tcp::OwnedReadHalf,
    tx_to_message_distributor: mpsc::Sender<ClientMessage>,
}

struct ClientWriter {
    id: u8,
    tx_stream: tokio::net::tcp::OwnedWriteHalf,
    rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
}

impl Client {
    fn new(
        id: u8,
        sock: TcpStream,
        tx_to_message_distributor: mpsc::Sender<ClientMessage>,
        rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
    ) -> Self {
        let (rx, tx) = sock.into_split();

        let reader = Arc::new(Mutex::new(ClientReader {
            id,
            rx_stream: rx,
            tx_to_message_distributor,
        }));
        let writer = Arc::new(Mutex::new(ClientWriter {
            id,
            tx_stream: tx,
            rx_from_message_distributor,
        }));

        Self { id, reader, writer }
    }

    async fn run(&self) {
        let mut client_join_set = JoinSet::new();
        client_join_set.spawn({
            let reader = self.reader.clone();
            async move {
                match reader.lock().await.handle_incoming().await {
                    Ok(_) => {}
                    Err(e) => tracing::error!("Handle incoming task has failed with error: {e}"),
                }
            }
        });
        client_join_set.spawn({
            let writer = self.writer.clone();
            async move {
                match writer.lock().await.handle_outgoing().await {
                    Ok(_) => {}
                    Err(e) => tracing::error!("Handle outgoing task has failed with error: {e}"),
                }
            }
        });
        client_join_set.join_next().await;

        tracing::warn!("Client {} died", self.id);
        // if either of the workers dies, we kill all the tasks
        // client object still will be alive though (cleanup to be added)
    }
}

impl Server {
    async fn bind(addr: &str) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        tracing::info!("Server started on {}, waiting for clients...", addr);

        let (tx_dist, rx_dist) = mpsc::channel(100);
        let (tx_broad, _) = broadcast::channel(16);

        // it MUST be run just before we start accepting clients
        task::spawn(Self::run_message_distributor(rx_dist, tx_broad.clone()));

        Ok(Self {
            listener,
            tx_to_message_distributor: tx_dist,
            tx_to_clients: tx_broad,
            connected_clients: HashMap::new(),
        })
    }

    async fn run(mut self) -> anyhow::Result<()> {
        let mut client_id_tracker: u8 = 0;

        loop {
            let (stream, socket_addr) = self.listener.accept().await?;

            tracing::info!(
                "New client {} connected from {}",
                client_id_tracker,
                socket_addr
            );

            self.tx_to_message_distributor
                .send(ClientMessage::connected(client_id_tracker))
                .await?;

            // TODO: Prepare thoroughout explanation, what is going on here
            // Think, If I can simplify this
            // Maybe this implementation gives me abilities which I don't need as for now
            let client = Arc::new(Client::new(
                client_id_tracker,
                stream,
                self.tx_to_message_distributor.clone(),
                self.tx_to_clients.subscribe(),
            ));
            // TODO: can I write shorter here?
            tokio::spawn({
                let client = client.clone();
                async move {
                    client.run().await;
                }
            });

            client_id_tracker += 1;

            self.connected_clients
                .insert(client_id_tracker, client.clone());
        }
    }

    async fn run_message_distributor(
        mut rx: mpsc::Receiver<ClientMessage>,
        tx_broad: broadcast::Sender<ClientMessage>,
    ) {
        while let Some(msg) = rx.recv().await {
            if tx_broad.send(msg).is_err() {
                tracing::error!("Failed to broadcast message - no receivers");
            }
        }
    }
}

impl ClientReader {
    async fn handle_incoming(&mut self) -> anyhow::Result<()> {
        loop {
            match NetworkMessage::read(&mut self.rx_stream).await {
                Ok(msg_net) => {
                    tracing::info!("Parsing message");
                    let mut msg_cli = ClientMessage::from_json(msg_net.get_payload())?;
                    msg_cli.set_id(self.id);

                    tracing::info!("Received message from client {}: {:?}", self.id, msg_cli);

                    if self.tx_to_message_distributor.send(msg_cli).await.is_err() {
                        tracing::error!("Failed to redistribute message from client {}", self.id);
                        break;
                    }
                }
                Err(e) => {
                    self.tx_to_message_distributor
                        .send(ClientMessage::disconnected(self.id))
                        .await?;

                    tracing::info!("Client {} disconnected: {}", self.id, e);

                    break;
                }
            }

            // match self.rx_stream.read_f32().await {
            //     Ok(msg) => {
            //         tracing::info!("Received message from client {}: {}", self.id, msg);

            //         let msg = ClientMessage::data(self.id, msg);
            //         if self.tx_to_message_distributor.send(msg).await.is_err() {
            //             tracing::error!("Failed to redistribute message from client {}", self.id);
            //             break;
            //         }
            //     }
            //     Err(e) => {
            //         self.tx_to_message_distributor
            //             .send(ClientMessage::disconnected(self.id))
            //             .await;

            //         tracing::info!("Client {} disconnected: {}", self.id, e);

            //         break;
            //     }
            // }
        }

        Ok(())
    }
}

impl ClientWriter {
    async fn handle_outgoing(&mut self) -> anyhow::Result<()> {
        while let Ok(msg) = self.rx_from_message_distributor.recv().await {
            if msg.get_id() == self.id {
                tracing::trace!(
                    "Ignore message destined to {}, we are: {}",
                    msg.get_id(),
                    self.id
                );
                continue;
            }

            let msg_json = msg.to_json()?;
            tracing::debug!("Sending {msg_json}");

            let msg_net = NetworkMessage::new(msg_json.as_bytes());
            tracing::debug!("Serialized view: |{:?}|", msg_net.get_payload());

            match self.tx_stream.write(&msg_net.get_payload()).await {
                Ok(_) => tracing::info!("Sent message {} to client {}", msg_json, self.id),
                Err(e) => {
                    tracing::error!("Failed to send message to client {}: {}", self.id, e);
                    break;
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let server = Server::bind("0.0.0.0:3456").await?;
    server.run().await
}
