use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::oneshot, sync::mpsc, time::Interval};
use std::{collections::HashMap, io::Write};
use crate::{command::Command, resp};
use std::collections::LinkedList;

pub async fn cmd_process(mut rx: mpsc::Receiver<Command>) {
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
            },
            Command::LLen { list_name, respond_to } => {
                match dict_list.get(&list_name) {
                    Some(list_name) => {
                        let _ = respond_to.send(list_name.len());
                    },
                    None => {let _ = respond_to.send(0);}
                }
            },
            Command::LPOP { list_name, respond_to } => {
                match dict_list.get_mut(&list_name) {
                    Some(ls) => {
                        let q = ls.pop_front();
                        match q {
                            Some(val) => {let _ = respond_to.send(val);},
                            None => {let _ = respond_to.send(b"".to_vec());}
                        }
                    },
                    None => {let _ = respond_to.send(b"".to_vec());}
                }
            }
        }
    }
}