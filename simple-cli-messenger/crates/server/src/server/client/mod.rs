mod reader;
mod timeout;
mod writer;

use super::ServerState;
use crate::server::client::reader::ClientReader;
use crate::server::client::timeout::Timeout;
use crate::server::client::writer::ClientWriter;

use ::tokio::sync::Mutex;
use anyhow::Context;
use protocol::client_message::ClientMessage;
use std::{sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc},
    task::JoinSet,
};

pub struct Client {
    pub username: String,
    reader: Arc<Mutex<ClientReader>>,
    writer: Arc<Mutex<ClientWriter>>,
}

impl Client {
    pub fn new(
        username: String,
        sock: TcpStream,
        tx_to_message_distributor: mpsc::Sender<ClientMessage>,
        rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
        server_state: Arc<ServerState>,
    ) -> Self {
        let (rx, tx) = sock.into_split();
        let (txx, rxx) = tokio::sync::mpsc::channel(100);

        let reader = Arc::new(Mutex::new(ClientReader::new(
            username.clone(),
            rx,
            tx_to_message_distributor,
            server_state,
            txx,
        )));
        let writer = Arc::new(Mutex::new(ClientWriter::new(
            username.clone(),
            tx,
            rx_from_message_distributor,
            rxx,
        )));

        Self {
            username: username.clone(),
            reader,
            writer,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        async fn handle_incoming_messages(reader: Arc<Mutex<ClientReader>>) -> anyhow::Result<()> {
            Ok(Timeout::new(
                Duration::from_secs(120),
                reader.lock().await.handle_incoming(),
            )
            .await
            .context("handle_incoming_messages exited")?)
        }

        async fn handle_outgoing_messages(writer: Arc<Mutex<ClientWriter>>) -> anyhow::Result<()> {
            Ok(writer
                .lock()
                .await
                .handle_outgoing()
                .await
                .context("handle_outgoing_messages exited")?)
        }

        let mut client_join_set = JoinSet::new();
        client_join_set.spawn(handle_incoming_messages(self.reader.clone()));
        client_join_set.spawn(handle_outgoing_messages(self.writer.clone()));

        // "unwrap" should be fine here as join set is guaranteed to be non-empty
        Ok(client_join_set.join_next().await.unwrap()??)
    }
}
