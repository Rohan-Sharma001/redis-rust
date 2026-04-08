use crate::{command::Command, resp};
use std::collections::LinkedList;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, VecDeque},
    io::Write,
    time::{self, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    sync::oneshot,
    time::Interval,
};

struct Waiter {
    deadline: Option<Instant>,
    respond_to: oneshot::Sender<Vec<u8>>,
}
impl Ord for Waiter {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.deadline, &other.deadline) {
            (None, None) => Ordering::Equal,
            (_, None) => Ordering::Less,
            (None, _) => Ordering::Greater,
            (a, b) => a.cmp(b),
        }
    }
}

impl PartialOrd for Waiter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Waiter {
    fn eq(&self, other: &Self) -> bool {
        &self.deadline == (&other.deadline)
    }
}
impl Eq for Waiter {}

pub async fn cmd_process(mut rx: mpsc::Receiver<Command>) {
    let mut dict: HashMap<Vec<u8>, (Vec<u8>, Option<std::time::Instant>)> = HashMap::new();
    let mut dict_list: HashMap<Vec<u8>, LinkedList<Vec<u8>>> = HashMap::new();
    let mut waiters: HashMap<Vec<u8>, BinaryHeap<Waiter>> = HashMap::new(); //store BLPOP requests

    loop {
        tokio::select! {
            biased;
            Some(cmd) = rx.recv() => {
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

                        match waiters.get_mut(&list_name) {
                            Some(dq) if dq.len() > 0 => {
                                let mut waiter_ = dq.pop().unwrap();
                                while let Some(exp_time) = waiter_.deadline && exp_time < Instant::now() {
                                    let _ = waiter_.respond_to.send(b"$-1\r\n".to_vec());
                                    waiter_ = match dq.pop() {
                                        Some(p) => p,
                                        None => return
                                    }
                                }
                                    let mut buffer = Vec::<u8>::new();
                                    write!(&mut buffer, "*2\r\n${}\r\n", list_name.len());
                                    buffer.extend_from_slice(&list_name);
                                    buffer.extend_from_slice(b"\r\n");
                                    let mut tmp_buff = Vec::<u8>::new();
                                    write!(&mut tmp_buff, "${}\r\n", dict_list.get(&list_name).unwrap().front().unwrap().len());
                                    tmp_buff.extend_from_slice(dict_list.get(&list_name).unwrap().front().unwrap());
                                    tmp_buff.extend_from_slice(b"\r\n");
                                    dict_list.get_mut(&list_name).unwrap().pop_front().unwrap();
                                    buffer.extend_from_slice(&tmp_buff);
                                    let k = waiter_.respond_to.send(buffer);
                            },
                            _ => {}
                        }
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
                        match waiters.get_mut(&list_name) {
                            Some(dq) if dq.len() > 0 => {
                                let mut waiter_ = dq.pop().unwrap();
                                while let Some(exp_time) = waiter_.deadline && exp_time < Instant::now() {
                                    let _ = waiter_.respond_to.send(b"$-1\r\n".to_vec());
                                    waiter_ = match dq.pop() {
                                        Some(p) => p,
                                        None => return
                                    }
                                }
                                    let mut buffer = Vec::<u8>::new();
                                    write!(&mut buffer, "*2\r\n${}\r\n", list_name.len());
                                    buffer.extend_from_slice(&list_name);
                                    buffer.extend_from_slice(b"\r\n");
                                    let mut tmp_buff = Vec::<u8>::new();
                                    write!(&mut tmp_buff, "${}\r\n", dict_list.get(&list_name).unwrap().front().unwrap().len());
                                    tmp_buff.extend_from_slice(dict_list.get(&list_name).unwrap().front().unwrap());
                                    tmp_buff.extend_from_slice(b"\r\n");
                                    dict_list.get_mut(&list_name).unwrap().pop_front().unwrap();
                                    buffer.extend_from_slice(&tmp_buff);
                                    let k = waiter_.respond_to.send(buffer);
                            },
                            _ => {}
                        }
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
                    Command::LPOP { list_name, no_of_elements, respond_to } => {
                        match dict_list.get_mut(&list_name) {
                            Some(ls) => {
                                if ls.len() == 0 {
                                    let _ = respond_to.send(b"$-1\r\n".to_vec());
                                    return;
                                }
                                let mut buf = Vec::<u8>::new();
                                match no_of_elements {
                                    Some(n) => {
                                        write!(&mut buf, "*{}\r\n", n);
                                        for i in 0..n {
                                            let p = ls.pop_front().unwrap();
                                            let mut tmp_buf = Vec::<u8>::new();
                                            write!(&mut tmp_buf, "${}\r\n", p.len());
                                            tmp_buf.extend_from_slice(p.as_slice());
                                            tmp_buf.extend_from_slice(b"\r\n");
                                            buf.extend_from_slice(&tmp_buf);
                                        }
                                    }
                                    None => {
                                        let q = ls.pop_front();
                                        match q {
                                            Some(vc) => {
                                                write!(&mut buf, "${}\r\n", vc.len());
                                                buf.extend_from_slice(vc.as_slice());
                                                buf.extend_from_slice(b"\r\n");
                                            }
                                            None => write!(&mut buf, "-1\r\n").unwrap()
                                        }
                                    }
                                }
                                let _ = respond_to.send(buf);
                            },
                            None => {let _ = respond_to.send(b"".to_vec());}
                        }
                    },
                    Command::BLPOP { list_name, exp_time, respond_to } => {
                        println!("laadle");
                        let now_ts = std::time::Instant::now();
                        let fin_time = match exp_time {
                            Some(e_t) => Some(now_ts + e_t),
                            None => None
                        };
                        println!("{:?}", fin_time);
                        match dict_list.get_mut(&list_name) {
                            Some(LL_) if LL_.len() > 0 => {
                                let nod = LL_.pop_front().unwrap();
                                let mut buffer = Vec::<u8>::new();
                                write!(&mut buffer, "{}\r\n", list_name.len());
                                buffer.extend_from_slice(&list_name);
                                buffer.extend_from_slice(b"\r\n");
                                let mut temp_buf = Vec::<u8>::new();
                                write!(&mut temp_buf, "{}\r\n", nod.len());
                                temp_buf.extend_from_slice(&nod);
                                temp_buf.extend_from_slice(b"\r\n");
                                buffer.extend_from_slice(&temp_buf);
                                respond_to.send(buffer);
                            }
                            _ => waiters.entry(list_name).or_insert(BinaryHeap::new()).push(Waiter{deadline: fin_time, respond_to})
                        }
                    }
                }
            },

            _ = tokio::time::sleep(time::Duration::from_millis(10)) => {
                // println!("tttt");
                let now_ = Instant::now();
                for x in waiters.iter_mut() {
                    loop {
                        let front_element = x.1.peek();
                        if let Some(waiter_) = front_element {
                            if waiter_.deadline < Some(now_) {
                                let front_ = x.1.pop().unwrap();
                                let _ = front_.respond_to.send(b"*-1\r\n".to_vec());
                                println!("A")
                            }
                            else {break;}
                        } else {break;}
                    }
                }
            }
        }
    }

    // while let Some(cmd) = rx.recv().await {

    // }
}
