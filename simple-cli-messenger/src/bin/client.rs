// Run this specific binary with:
// cargo run --bin server

use bytes::{Buf, BufMut, BytesMut};
use tokio::{
    self,
    io::{self, AsyncReadExt, AsyncWriteExt, unix::AsyncFdTryNewError},
    net::{self, TcpListener, TcpStream},
    spawn,
};

// апка приймає наступні аргументи:
// chat --server port
// chat --client ip:port

// вимоги:
// всі важливі події логуються (tracing)
// комунікація відбувається через ТСР сокет (tokio)
// сервер лише отримує повідомлення від клієнтів і розсилає його всім іншим
// клієнти відсилають всі повідомлення з stdin на сервер
// клієнти показують всі повідомлення від інших клієнтів в stdout (можна теж логами)

/*
 * server binds to ip & port
 * clients can connect to server (before connecting to the server ask user its name);
 *     once connected they can start receiving all messages sent by other users (basically, server would need to send that info to them)
 *
 */
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

    // can't use tokio::join here as it "evaluates them concurrently on the same task" - meaning that we would stuck on user input
    get_user_input_task.await.unwrap();
    get_server_messages_task.await.unwrap();
}
