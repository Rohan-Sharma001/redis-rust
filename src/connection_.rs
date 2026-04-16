use core::time;
use std::{io::Write, time::Duration};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::oneshot, sync::mpsc, time::Interval};
use crate::command::Command;
use crate::resp::value::DataObjects;
use crate::resp::decode::*;

pub async fn connection_(mut socket: TcpStream, mut tx: mpsc::Sender<Command>) {
    let mut buffer = [0; 4096];
    loop {
        let input_bytes = socket.read(&mut buffer).await.unwrap();
        if input_bytes == 0 {
            // EEEEEEEE
            break;
        }
        let mut iterator_var = 0;
        let decoded_val = match resp_decode_value(&buffer, &mut iterator_var) {
            Some(data) => data,
            None => break
        };
        println!("{}", decoded_val);
        match decoded_val {
            DataObjects::BasicString(val) => {
                if val == "PING" {
                    // socket.write_all(b"PONG").await.unwrap();
                }
            }
            DataObjects::Array(Some(arr)) => {
                if arr[0].as_command() == Some(b"ECHO") {
                    let value = format!("{}", arr[1]);
                    // socket.write_all(format!("${}\r\n{}\r\n", str_to_write.len(), str_to_write).as_bytes()).await.unwrap();
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::Echo { value: value.as_bytes().to_vec(), respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();

                    let bs = [format!("${}\r\n", res.len()).as_bytes() , res.as_slice() , b"\r\n"].concat();
                    socket.write_all(bs.as_slice()).await.unwrap();
                } else if arr[0].as_command() == Some(b"PING") {
                    socket.write_all(b"+PONG\r\n").await.unwrap();
                } else if arr[0].as_command() == Some(b"SET") {
                    // if arr.len() != 3 {
                    //     println!("Expected 2 argument");
                    // }
                    if let Some(key) = arr[1].as_command() && let Some(value) = arr[2].as_command() {
                        let (response_tx, response_rx) = oneshot::channel();
                        let cmd;
                        if arr.len() >= 5 {
                            let duration_: f32 = str::from_utf8(arr[4].as_command().unwrap()).unwrap().parse().unwrap();
                            match arr[3].as_command() {
                                Some(b"EX") => {
                                    cmd = Command::Set { key: key.to_vec(), value: value.to_vec(), life: Some(std::time::Duration::from_secs_f32(duration_)), respond_to: response_tx };
                                },
                                Some(b"PX") => {
                                    cmd = Command::Set { key: key.to_vec(), value: value.to_vec(), life: Some(std::time::Duration::from_millis(duration_ as u64)), respond_to: response_tx };
                                },
                                _ => {
                                    cmd = Command::Set { key: key.to_vec(), value: value.to_vec(), life: None, respond_to: response_tx };
                                }
                            }
                        } else {
                            cmd = Command::Set { key: key.to_vec(), value: value.to_vec(), life: None, respond_to: response_tx };
                        }

                        // let cmd = Command::Set { key: key.to_vec(), value: value.to_vec(), respond_to: response_tx };
                        let _ = tx.send(cmd).await;
                        let res = response_rx.await.unwrap();

                        socket.write_all(b"+OK\r\n").await.unwrap();
                    }
                } else if arr[0].as_command() == Some(b"GET") {
                    let (response_tx, response_rx) = oneshot::channel();
                    if let Some(key) = arr[1].as_command() {
                        let cmd = Command::Get { key: key.to_vec(), respond_to: response_tx };
                        let _ = tx.send(cmd).await;
                        let res = response_rx.await.unwrap();
                        match res {
                            Some(res) => {
                                let bs = [format!("${}\r\n", res.len()).as_bytes() , res.as_slice() , b"\r\n"].concat();
                                socket.write_all(bs.as_slice()).await.unwrap();
                            },
                            None => {
                                socket.write_all(b"$-1\r\n").await.unwrap();
                            }
                        }
                    }
                } else if arr[0].as_command() == Some(b"RPUSH") && arr.len() > 2 {
                    let list_name = format!("{}", arr[1]);
                    let mut value_list = Vec::<Vec<u8>>::new();
                    for i in 2..arr.len() {
                        value_list.push(arr[i].as_command().unwrap().to_vec());
                    }
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::RPUSH { list_name: list_name.as_bytes().to_vec(), value_list: value_list, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    socket.write_all(format!(":{}\r\n", res).as_bytes()).await.unwrap();
                    
                }else if arr[0].as_command() == Some(b"LPUSH") && arr.len() > 2 {
                    let list_name = format!("{}", arr[1]);
                    let mut value_list = Vec::<Vec<u8>>::new();
                    for i in 2..arr.len() {
                        value_list.push(arr[i].as_command().unwrap().to_vec());
                    }
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::LPUSH { list_name: list_name.as_bytes().to_vec(), value_list: value_list, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    socket.write_all(format!(":{}\r\n", res).as_bytes()).await.unwrap();
                } else if arr[0].as_command() == Some(b"LRANGE") && arr.len() > 3 {
                    let list_name = arr[1].as_command().clone().unwrap().to_vec();
                    let start_index: i32 = str::from_utf8(arr[2].as_command().unwrap()).unwrap().parse().unwrap();
                    let end_index: i32 = str::from_utf8(arr[3].as_command().unwrap()).unwrap().parse().unwrap();
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::LRange { list_name: list_name, start_index, end_index, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    socket.write_all(res.as_slice()).await.unwrap();
                } else if arr[0].as_command() == Some(b"LLEN") && arr.len() > 1 {
                    let list_name = arr[1].as_command().clone().unwrap().to_vec();
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::LLen { list_name, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    let mut buf = Vec::<u8>::new();
                    write!(&mut buf, ":{}\r\n", res);
                    socket.write_all(&buf).await.unwrap();
                } else if arr[0].as_command() == Some(b"LPOP") && arr.len() > 1 {
                    let list_name = arr[1].as_command().clone().unwrap().to_vec();
                    let (response_tx, response_rx) = oneshot::channel();
                    
                    let no_of_elements = match arr.get(2) {
                        Some(e) => Some(str::from_utf8(e.as_command().unwrap()).unwrap().parse::<i32>().unwrap()),
                        None => None,
                    };
                    let cmd  = Command::LPOP { list_name, no_of_elements: no_of_elements, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    socket.write_all(&res).await.unwrap();
                } else if arr[0].as_command() == Some(b"BLPOP") && arr.len() > 1 {
                    let list_name = arr[1].as_command().clone().unwrap().to_vec();
                    let (response_tx, response_rx) = oneshot::channel();
                    
                    let exp_time = match str::from_utf8(arr[2].as_command().
                    unwrap())
                    .unwrap()
                    .parse::<f32>()
                    .unwrap() {
                        0.0 => None,
                        s => Some(Duration::from_secs_f32(s))
                    }; 
                    let cmd  = Command::BLPOP { list_name, exp_time, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    
                    let res = response_rx.await.unwrap();
                    socket.write_all(&res).await.unwrap();
                    println!("bbbb");
                } else if arr[0].as_command() == Some(b"TYPE") {
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::TYPE { key: arr[1].as_command().unwrap().to_vec(), respond_to:  response_tx};
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    socket.write_all(&res).await.unwrap();
                } else if arr[0].as_command() == Some(b"XADD") {
                    let (response_tx, response_rx) = oneshot::channel();
                    let mut kv_: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
                    for i in 3..arr.len()-1 {
                        kv_.push((arr[i].as_command().unwrap().to_vec(), arr[i+1].as_command().unwrap().to_vec()));
                    }
                    let cmd = Command::XADD { key: arr[1].as_command().unwrap().to_vec(), stream_id: arr[2].as_command().unwrap().to_vec(), value_pairs: kv_, respond_to: response_tx };
                    let _ = tx.send(cmd).await;
                    let res = response_rx.await.unwrap();
                    socket.write_all(&res).await.unwrap();
                }
            }
            _ => {},
        }
    }
}
