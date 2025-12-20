mod client;

use crate::server::client::Client;

use protocol::client_message::ClientMessage;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    task::{self},
};

pub struct Server {
    listener: TcpListener,
    tx_to_message_distributor: mpsc::Sender<ClientMessage>,
    tx_to_clients: broadcast::Sender<ClientMessage>,
    connected_clients: HashMap<String, Arc<Client>>,
}

impl Server {
    pub async fn bind(addr: &str) -> anyhow::Result<Self> {
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

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            let (mut stream, socket_addr) = self.listener.accept().await?;

            let username = match self.retrieve_client_username(&mut stream).await {
                Err(e) => {
                    tracing::error!("Failed to retrieve client username: {e}");
                    continue;
                }
                Ok(username) => username,
            };

            tracing::info!("[{username}] connected from {socket_addr}");

            self.tx_to_message_distributor
                .send(ClientMessage::connected(username.clone()))
                .await?;

            // TODO: Prepare thoroughout explanation, what is going on here
            // Think, If I can simplify this
            // Maybe this implementation gives me abilities which I don't need as for now
            let client = Arc::new(Client::new(
                username.clone(),
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

            self.connected_clients.insert(username, client.clone());
        }
    }
}

/////////////////////////////
// Private
/////////////////////////////
impl Server {
    async fn retrieve_client_username(&mut self, stream: &mut TcpStream) -> anyhow::Result<String> {
        let msg_cli = ClientMessage::read(stream).await?;
        let ClientMessage::Connected(common) = msg_cli else {
            return Err(anyhow::anyhow!(
                "Unexpected first message from the client: {:?}.",
                msg_cli
            ));
        };

        return Ok(common.username);
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
