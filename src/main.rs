#![allow(unused_imports)]
use std::{io::{Read, Write}, net::{TcpListener, TcpStream}};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment the code below to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    //
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                // _stream.write_all(b"+PONG\r\n").unwrap();
                let mut buffer = [0; 4096];
                loop {
                    let n = _stream.read(&mut buffer).unwrap();
                    if n == 0 {
                        // EEEEEEEEEEEEEEEEEEEEEEEE
                        break;
                    }
                    if buffer.starts_with(b"*1\r\n$4\r\nPING\r\n")  {
                         _stream.write_all(b"+PONG\r\n").unwrap();
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
