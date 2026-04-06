#![allow(unused_imports)]
use core::fmt;
use std::{array, collections::{HashMap, LinkedList}, ffi::os_str::Display, fmt::write, hash::Hash, io::{Read, Write}, num::ParseIntError, ptr::null, time::Duration, vec};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::oneshot, sync::mpsc, time::Interval};
use codecrafters_redis::connection_::*;
use codecrafters_redis::store::*;

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");
    // let mut main_dict: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(cmd_process(rx));
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
        let tx_copy = tx.clone();
        let (socket, addr) = listener.accept().await.unwrap();
        println!{"Connected to {}", addr};
        // connection_(socket).await;
        tokio::spawn(async move {
            connection_(socket, tx_copy).await
        });
    }
    // let mut iterator_var = 0;
    // let test = resp_decode_value(b"*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n", &mut iterator_var);
    // match test {
    //     Some(test) => println!("{}", test),
    //     None => {},
    // }
}



