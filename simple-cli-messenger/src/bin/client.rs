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

#[tokio::main]
async fn main() {
    // TODO: user MUST enter its name before joining the chat

    let addr = "127.0.0.1:3456".parse().unwrap();
    let client = tokio::net::TcpSocket::new_v4().unwrap();
    let stream = client.connect(addr).await.unwrap();

    let (mut rx, mut tx) = stream.into_split();

    let get_user_input_task = spawn(async move {
        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            println!("User input: {input}");

            match input.trim().parse::<f32>() {
                Ok(num) => tx.write_f32(num).await.unwrap(),
                Err(_) => {
                    println!("Invalid number, try again.");
                }
            }
        }
    });

    let get_server_messages_task = spawn(async move {
        loop {
            match rx.read_f32().await {
                Ok(num) => println!("Received message: {num}"),
                Err(e) => println!("Error: {e}"),
            }
        }
    });

    get_user_input_task.await.unwrap();
    get_server_messages_task.await.unwrap();
}
