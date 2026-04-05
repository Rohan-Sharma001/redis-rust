#![allow(unused_imports)]
use core::fmt;
use std::{array, collections::{HashMap, LinkedList}, ffi::os_str::Display, fmt::write, hash::Hash, io::{Read, Write}, num::ParseIntError, ptr::null, time::Duration, vec};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::oneshot, sync::mpsc, time::Interval};

enum Command {
    Get {
        key: Vec<u8>,
        respond_to: oneshot::Sender<Option<Vec<u8>>>,
    },
    Set {
        key: Vec<u8>,
        value: Vec<u8>,
        life: Option<std::time::Duration>,
        respond_to: oneshot::Sender<Option<()>>,
    },
    Echo {
        value: Vec<u8>,
        respond_to: oneshot::Sender<Vec<u8>>,
    },
    RPUSH {
        list_name: Vec<u8>,
        value_list: Vec<Vec<u8>>,
        respond_to: oneshot::Sender<usize>
    },
    LRange {
        list_name: Vec<u8>,
        start_index: i32,
        end_index: i32,
        respond_to: oneshot::Sender<Vec<u8>>
    },
    LPUSH {
        list_name: Vec<u8>,
        value_list: Vec<Vec<u8>>,
        respond_to: oneshot::Sender<usize>
    }
}

#[derive(PartialEq)]
enum DataObjects {
    BasicString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<DataObjects>>)
}

impl DataObjects {
    fn as_command(&self) -> Option<&[u8]> {
        match self {
            DataObjects::BasicString(str) => Some(str.as_bytes()),
            DataObjects::Error(str) => Some(str.as_bytes()),
            DataObjects::BulkString(Some(vc)) => Some(vc),
            _ => None,
        }
    }
}

impl fmt::Display for DataObjects {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DataObjects::BasicString(str) => write!(f, "{}", str),
            DataObjects::Error(err) => write!(f, "{}", err),
            DataObjects::Integer(int) => write!(f, "{}", int),
            DataObjects::BulkString(Some(arr)) => write!(f, "{}", String::from_utf8(arr.clone()).unwrap()),
            // DataObjects::BulkString(Some(arr)) => write!(f, "{:?}", arr),
            DataObjects::Array(Some(obj_arr)) => {
                write!(f, "[ ");
                for x in obj_arr {
                    write!(f, "\x08");
                    write!(f, "{}, ", x);
                }
                write!(f, "\x08\x08");
                write!(f, "]");
                Ok(())
            },
            _ => {Ok(())},
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");
    let mut main_dict: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

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
        let (mut socket, addr) = listener.accept().await.unwrap();
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

async fn connection_(mut socket: TcpStream, mut tx: mpsc::Sender<Command>) {
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
                    tx.send(cmd).await.unwrap();
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
                        tx.send(cmd).await.unwrap();
                        let res = response_rx.await.unwrap();

                        socket.write_all(b"+OK\r\n").await.unwrap();
                    }
                } else if arr[0].as_command() == Some(b"GET") {
                    let (response_tx, response_rx) = oneshot::channel();
                    if let Some(key) = arr[1].as_command() {
                        let cmd = Command::Get { key: key.to_vec(), respond_to: response_tx };
                        tx.send(cmd).await.unwrap();
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
                    tx.send(cmd).await.unwrap();
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
                    tx.send(cmd).await.unwrap();
                    let res = response_rx.await.unwrap();
                    socket.write_all(format!(":{}\r\n", res).as_bytes()).await.unwrap();
                } else if arr[0].as_command() == Some(b"LRANGE") && arr.len() > 3 {
                    let list_name = arr[1].as_command().clone().unwrap().to_vec();
                    let start_index: i32 = str::from_utf8(arr[2].as_command().unwrap()).unwrap().parse().unwrap();
                    let end_index: i32 = str::from_utf8(arr[3].as_command().unwrap()).unwrap().parse().unwrap();
                    let (response_tx, response_rx) = oneshot::channel();
                    let cmd = Command::LRange { list_name: list_name, start_index, end_index, respond_to: response_tx };
                    tx.send(cmd).await.unwrap();
                    let res = response_rx.await.unwrap();
                    socket.write_all(res.as_slice()).await.unwrap();
                }
            }
            _ => {},
        }
    }
}

fn resp_decode_value(byte_array: &[u8], iterator_var: &mut usize) -> Option<DataObjects> {
    // let mut data_array: Vec<DataObjects> = Vec::new();
    // let mut iterator_var = 0;
    let n = byte_array.len();
    while (*iterator_var < n) {
        match byte_array[*iterator_var] as char {
            '+'  => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let new_val = DataObjects::BasicString(str::from_utf8(&byte_array[*iterator_var+1..iterator_end]).unwrap().to_string());
                *iterator_var=iterator_end+2;
                return Some(new_val);
            },
            '-' => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let new_val = DataObjects::Error(str::from_utf8(&byte_array[*iterator_var+1..iterator_end]).unwrap().to_string());
                *iterator_var=iterator_end+2;
                return Some(new_val);
            },
            ':' => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let subslice = &byte_array[*iterator_var+1..iterator_end];
                let new_val = DataObjects::Integer(str::from_utf8(subslice).unwrap().parse().unwrap());
                *iterator_var=iterator_end+2;
                return Some(new_val);
            },
            '$' => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let subslice = &byte_array[*iterator_var+1..iterator_end];
                let length_buffer: i32 = str::from_utf8(subslice).unwrap().parse().unwrap();
                let new_val = match length_buffer{
                    -1 => DataObjects::BulkString(None),
                    _ => DataObjects::BulkString(Some(byte_array[iterator_end+2..iterator_end+2+length_buffer as usize].to_vec()))
                };
                *iterator_var=iterator_end + 4 + length_buffer as usize;
                return Some(new_val);
            },
            '*' => {

                return resp_decode_array(byte_array, iterator_var);
            },
            _ => {
                println!("TF?????, {}", iterator_var);
            }
        }
    }
    return None;
}

fn resp_decode_array(byte_array: &[u8], iterator_var: &mut usize) -> Option<DataObjects> {
    let mut vc: Vec<DataObjects> = Vec::new();
    let mut iterator_end = *iterator_var;
    let n = byte_array.len();
    while (iterator_end < n-1) {
        if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
        else {iterator_end+=1;}
    }
    let subslice = &byte_array[*iterator_var+1..iterator_end];
    let length_buffer: i32 = str::from_utf8(subslice).unwrap().parse().unwrap();
    *iterator_var = iterator_end+2;
    if length_buffer == 0 {
        return Some(DataObjects::Array(Some(vc)));
    } else if length_buffer < 0 {
        return Some(DataObjects::Array(None));
    }
    for i in 0..length_buffer {
        let val = resp_decode_value(byte_array, iterator_var);
        if let Some(x) = val {
            vc.push(x);
        }
    }
    return Some(DataObjects::Array(Some(vc)));
}

async fn cmd_process(mut rx: mpsc::Receiver<Command>) {
    let mut dict: HashMap<Vec<u8>, (Vec<u8>, Option<std::time::Instant>)> = HashMap::new();
    let mut dict_list: HashMap<Vec<u8>, LinkedList<Vec<u8>>> = HashMap::new();
    while let Some(cmd) = rx.recv().await {
        let now_ts = std::time::Instant::now();
        match cmd {
            Command::Get {key, respond_to}=> {
                let val_pair = dict.get(&key).cloned();
                match val_pair {
                    Some((val, Some(ts))) => {
                        println!("{:?}....{:?}", now_ts, ts);
                        if now_ts > ts {
                            let _ = respond_to.send(None);
                        } else {
                            let _ = respond_to.send(Some(val));
                        }
                    },
                    Some((val, None)) => {
                        let _ = respond_to.send(Some(val));
                    }
                    None => {
                        let _ = respond_to.send(None);
                    }
                }
            },
            Command::Set {key, value, life, respond_to} => {
                // dict.insert(key, (value, now_ts + life));
                match life {
                    Some(life) => dict.insert(key, (value, Some(now_ts + life))),
                    None => dict.insert(key, (value, None)),
                };
                let _ = respond_to.send(Some(()));
            },
            Command::Echo {value, respond_to} => {
                let _ = respond_to.send(value);
            },
            Command::RPUSH { list_name, value_list, respond_to } => {
                dict_list.entry(list_name.clone()).or_insert(LinkedList::new());
                for str_ in value_list {
                    dict_list.entry(list_name.clone()).or_default().push_back(str_);
                }
                for x in dict_list.get(&list_name) {
                    println!("{:?}", x);
                }
                let _ = respond_to.send(dict_list.entry(list_name.clone()).or_default().len());
            },
            Command::LPUSH { list_name, value_list, respond_to } => {
                dict_list.entry(list_name.clone()).or_insert(LinkedList::new());
                for str_ in value_list {
                    dict_list.entry(list_name.clone()).or_default().push_front(str_);
                }
                for x in dict_list.get(&list_name) {
                    println!("{:?}", x);
                }
                let _ = respond_to.send(dict_list.entry(list_name.clone()).or_default().len());
            },
            Command::LRange { list_name, mut start_index, mut end_index, respond_to } => {
                match dict_list.get(&list_name) {
                    Some(list) => {
                        let n = list.len() as i32;
                        if start_index < -1*n {
                            start_index = 0;
                        }
                        if end_index < -1*n {
                            end_index = 0;
                        }
                        if end_index >= n {
                            end_index = n-1;
                        }
                        if start_index >= list.len() as i32 {
                            let _ = respond_to.send(b"*0\r\n".to_vec());
                            return;
                        }
                        start_index = ((start_index % n)+n)%n;
                        end_index = ((end_index % n)+n)%n;
                        if start_index > end_index {
                            let _ = respond_to.send(b"*0\r\n".to_vec());
                        } else {
                            // let e_index = std::cmp::min(end_index, (list.len()-1) as i32);
                            let mut return_str = Vec::<u8>::new();
                            write!(&mut return_str, "*{}\r\n", end_index - start_index + 1).unwrap();
                            for (ind,it) in list.iter().enumerate() {
                                if ind as i32 > end_index {break};
                                if ind as i32 >= start_index {
                                    let mut tmp_buf: Vec<u8> = Vec::new();
                                    write!(&mut tmp_buf, "${}\r\n", it.len()).unwrap();
                                    tmp_buf.extend_from_slice(it);
                                    tmp_buf.extend_from_slice(b"\r\n");
                                    return_str.extend_from_slice(&tmp_buf);
                                }
                            }
                            let _ = respond_to.send(return_str);
                        }
                    },
                    None => {
                        let _ = respond_to.send(b"*0\r\n".to_vec());
                    }
                }
            }
        }
    }
}