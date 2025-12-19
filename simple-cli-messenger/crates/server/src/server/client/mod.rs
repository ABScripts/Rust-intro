mod reader;
mod timeout;
mod writer;

use crate::server::client::reader::ClientReader;
use crate::server::client::timeout::Timeout;
use crate::server::client::writer::ClientWriter;

use ::tokio::sync::Mutex;
use protocol::client_message::ClientMessage;
use std::{sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc},
    task::JoinSet,
};

pub struct Client {
    username: String,
    reader: Arc<Mutex<ClientReader>>,
    writer: Arc<Mutex<ClientWriter>>,
}

impl Client {
    pub fn new(
        username: String,
        sock: TcpStream,
        tx_to_message_distributor: mpsc::Sender<ClientMessage>,
        rx_from_message_distributor: broadcast::Receiver<ClientMessage>,
    ) -> Self {
        let (rx, tx) = sock.into_split();

        let reader = Arc::new(Mutex::new(ClientReader::new(
            username.clone(),
            rx,
            tx_to_message_distributor,
        )));
        let writer = Arc::new(Mutex::new(ClientWriter::new(
            username.clone(),
            tx,
            rx_from_message_distributor,
        )));

        Self {
            username: username.clone(),
            reader,
            writer,
        }
    }

    pub async fn run(&self) {
        let handle_incoming_messages = Timeout::new(Duration::from_secs(6), {
            let reader = self.reader.clone();
            async move {
                match reader.lock().await.handle_incoming().await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Handle incoming task has failed with error: {e}")
                    }
                }
            }
        });

        let handle_outgoing_messages = {
            let writer = self.writer.clone();
            async move {
                match writer.lock().await.handle_outgoing().await {
                    Ok(_) => {}
                    Err(e) => tracing::error!("Handle outgoing task has failed with error: {e}"),
                }
            }
        };

        let mut client_join_set = JoinSet::new();
        client_join_set.spawn(handle_incoming_messages);
        client_join_set.spawn(handle_outgoing_messages);
        client_join_set.join_next().await;

        tracing::warn!("Client {} has died", self.username);
        // if either of the workers dies, we kill all the tasks
        // client object still will be alive though (cleanup to be added)
    }
}
