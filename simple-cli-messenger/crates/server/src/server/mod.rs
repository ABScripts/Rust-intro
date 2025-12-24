mod client;

use crate::server::client::Client;

use protocol::client_message::ClientMessage;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, RwLock, broadcast, mpsc},
    task::{self},
};

#[derive(Default)]
struct ServerState {
    connected_clients: Arc<RwLock<HashMap<String, Arc<Client>>>>,
}

pub struct Server {
    listener: TcpListener,
    tx_to_message_distributor: mpsc::Sender<ClientMessage>,
    tx_to_clients: broadcast::Sender<ClientMessage>,

    state: Arc<ServerState>,
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
            state: Arc::new(ServerState::default()),
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
            self.handle_client_connection(username, stream).await?;
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

        Ok(common.username)
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

    async fn handle_client_connection(
        self: &Self,
        username: String,
        stream: TcpStream,
    ) -> anyhow::Result<()> {
        self.tx_to_message_distributor
            .send(ClientMessage::connected(username.clone()))
            .await?;

        let client = Arc::new(Client::new(
            username.clone(),
            stream,
            self.tx_to_message_distributor.clone(),
            self.tx_to_clients.subscribe(),
            self.state.clone(),
        ));

        self.state
            .connected_clients
            .write()
            .await
            .insert(username, client.clone());

        Ok(self.run_client_worker(client))
    }

    fn run_client_worker(self: &Self, client: Arc<Client>) {
        let tx_to_message_distributor = self.tx_to_message_distributor.clone();
        let server_state = self.state.clone();

        tokio::spawn(async move {
            match client.run().await {
                Err(e) => tracing::error!("{e}"),
                _ => {}
            };

            let _ = tx_to_message_distributor
                .send(ClientMessage::disconnected(client.username.clone()))
                .await;

            server_state
                .connected_clients
                .write()
                .await
                .remove(&client.username);
        });
    }
}
