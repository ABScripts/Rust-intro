use bytes::{Buf, BufMut, BytesMut};
use tokio::{
    self,
    io::{self, AsyncReadExt, AsyncWriteExt, unix::AsyncFdTryNewError},
    net::{self, TcpListener, TcpStream},
    spawn,
};

use std::sync::{Arc, Mutex};
use std::vec::Vec;
use std::{cell::RefCell, str::FromStr};
use std::{io::LineWriter, rc::Rc};

struct ClientReader {
    id: u8,
    rx_stream: net::tcp::OwnedReadHalf, // used to receive messages from clients
    tx_redistribute_msg: tokio::sync::mpsc::Sender<bytes::BytesMut>, // used to send messages to message distributor
}

struct ClientWriter {
    id: u8,
    tx_stream: net::tcp::OwnedWriteHalf,
}

async fn simulate_server_side() -> io::Result<()> {
    let listener = net::TcpListener::bind("0.0.0.0:3456").await?;
    println!("Server started, waiting for new clients");

    let (tx_dist, mut rx) = tokio::sync::mpsc::channel(100);
    // it will yield error until we really use the channel: Clone something, bla bla
    let (tx_broad, _) = tokio::sync::broadcast::channel(16);

    let accept_new_clients_task = spawn({
        let tx_broad = tx_broad.clone();

        async move {
            let mut client_id_tracker: u8 = 0;
            loop {
                let (stream, _socket_addr) = listener.accept().await.unwrap();

                println!("New client connected!");

                let (rx, tx) = stream.into_split();

                let mut client_writer = ClientWriter {
                    id: client_id_tracker,
                    tx_stream: tx,
                };

                let mut client_reader = ClientReader {
                    id: client_id_tracker,
                    rx_stream: rx,
                    tx_redistribute_msg: tx_dist.clone(),
                };

                client_id_tracker += 1;

                // wait for new messages from connected clients
                spawn(async move {
                    loop {
                        let msg = client_reader.rx_stream.read_f32().await.unwrap();

                        // TODO: handle client disconnection
                        // I also need to cancel async tasks which handle tx halves of the connected clients.

                        println!("Received message from client {}!", client_reader.id);

                        let mut msg_buf = bytes::BytesMut::new();
                        msg_buf.put_u8(client_reader.id);
                        msg_buf.put_f32(msg);

                        client_reader
                            .tx_redistribute_msg
                            .send(msg_buf)
                            .await
                            .unwrap();
                    }
                });

                // distribute messages from other connected clients
                // TODO: join handle of this task MUST be added to some vector of tx halves join handles
                spawn({
                    let mut rx_broad: tokio::sync::broadcast::Receiver<BytesMut> =
                        tx_broad.subscribe();

                    async move {
                        while let Ok(mut msg) = rx_broad.recv().await {
                            let id = msg.get_u8();
                            let m = msg.get_f32();
                            println!("Try distributing message {m} to all other clients");

                            if id != client_writer.id {
                                println!("Sending {m} to client {id}");
                                client_writer.tx_stream.write_f32(m).await.unwrap();
                            }
                        }
                    }
                });
            }
        }
    });

    // this distributor is needed as we have multiple consumers in broadcast channels
    // otherwise we could have directly send messages from client rx tasks;
    let distribute_received_messages_to_other_connected_clients_task = spawn(async move {
        while let Some(msg) = rx.recv().await {
            tx_broad.send(msg).unwrap();
        }
    });

    accept_new_clients_task.await?;
    distribute_received_messages_to_other_connected_clients_task.await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::join!(simulate_server_side());
}
