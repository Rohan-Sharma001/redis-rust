#![allow(unused_imports)]
use std::{io::{Read, Write}};
use tokio::{io::AsyncWriteExt, net::{TcpListener, TcpStream}};
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    //
    // for stream in listener.incoming() {
    //     match stream {
    //         Ok(mut _stream) => {
    //             let mut buffer = [0; 4096];
    //             loop {
    //                 let n = _stream.read(&mut buffer).unwrap();
    //                 if n == 0 {
    //                     // EEEEEEEEEEEEEEEEEEEEEEEE
    //                     break;
    //                 }
    //                 if buffer.starts_with(b"*1\r\n$4\r\nPING\r\n")  {
    //                      _stream.write_all(b"+PONG\r\n").unwrap();
    //                 }
    //             }
    //         }
    //         Err(e) => {
    //             println!("error: {}", e);
    //         }
    //     }
    // }
    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        println!{"Connected to {}", addr};
        tokio::spawn(async move {
            let mut buffer = [0; 4096];
            loop {
                let input_bytes = socket.read(&mut buffer).await.unwrap();
                if input_bytes == 0 {
                    // EEEEEEEE
                    break;
                }
                if buffer.starts_with(b"*1\r\n$4\r\nPING\r\n") {
                    socket.write_all(b"+PONG\r\n").await.unwrap();
                    println!("PONG");
                }
            }
        });
    }
}
