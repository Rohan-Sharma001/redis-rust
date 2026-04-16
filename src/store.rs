use crate::{
    radx,
    resp::Data_Storage::Data_Storage::{ self, RedisList, RedisString, RedisStream },
};
use crate::{ command::Command };
use std::collections::LinkedList;
use std::{
    cmp::Ordering,
    collections::{ BinaryHeap, HashMap },
    io::Write,
    time::{ self, Instant },
};
use tokio::{ sync::mpsc, sync::oneshot };

struct Waiter {
    deadline: Option<Instant>,
    respond_to: oneshot::Sender<Vec<u8>>,
}
impl Ord for Waiter {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.deadline, &other.deadline) {
            (None, None) => Ordering::Equal,
            (_, None) => Ordering::Greater,
            (None, _) => Ordering::Less,
            (a, b) => b.cmp(a),
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
        &self.deadline == &other.deadline
    }
}
impl Eq for Waiter {}

pub async fn cmd_process(mut rx: mpsc::Receiver<Command>) {
    let mut dict: HashMap<Vec<u8>, (Data_Storage, Option<std::time::Instant>)> = HashMap::new();
    // let mut dict_list: HashMap<Vec<u8>, LinkedList<Vec<u8>>> = HashMap::new();
    let mut waiters: HashMap<Vec<u8>, BinaryHeap<Waiter>> = HashMap::new(); //store BLPOP requests
    // let mut streams_ = radx::RadixT::new();

    loop {
        tokio::select! {
            biased;
            Some(cmd) = rx.recv() => {
                let now_ts = std::time::Instant::now();
                match cmd {
                    Command::Get {key, respond_to}=> {
                        let val_pair = dict.get(&key);
                        match val_pair {
                            Some((RedisString(val), Some(ts))) => {
                                println!("{:?}....{:?}", now_ts, ts);
                                if now_ts > *ts {
                                    let _ = respond_to.send(None);
                                } else {
                                    let _ = respond_to.send(Some((*val).clone()));
                                }
                            },
                            Some((RedisString(val), None)) => {
                                let _ = respond_to.send(Some((*val).clone()));
                            }
                            _ => {
                                let _ = respond_to.send(None);
                            }
                        }
                    },
                    Command::Set {key, value, life, respond_to} => {
                        // dict.insert(key, (value, now_ts + life));
                        match life {
                            Some(life) => dict.insert(key, (RedisString(value), Some(now_ts + life))),
                            None => dict.insert(key, (RedisString(value), None)),
                        };
                        let _ = respond_to.send(Some(()));
                    },
                    Command::Echo {value, respond_to} => {
                        let _ = respond_to.send(value);
                    },
                    Command::RPUSH { list_name, value_list, respond_to } => {
                        // Check if list exists (and type)
                        let entry_ = dict.entry(list_name.clone()).or_insert((RedisList(LinkedList::new()), None));
                        if let Some((RedisList(rl_), _)) = dict.get_mut(&list_name) {
                            for str_ in value_list {
                                rl_.push_back(str_);
                            }
                            let _ = respond_to.send(rl_.len());
                        }
                        else {
                            respond_to.send(0);
                            continue;
                        }
                    },
                    Command::LPUSH { list_name, value_list, respond_to } => {
                        // Check if list exists (and type)
                        let entry_ = dict.entry(list_name.clone()).or_insert((RedisList(LinkedList::new()), None));
                        if let Some((RedisList(rl_), _)) = dict.get_mut(&list_name) {
                            for str_ in value_list {
                                rl_.push_front(str_);
                            }
                            let _ = respond_to.send(rl_.len());
                        }
                        else {
                            respond_to.send(0);
                            continue;
                        }
                    },
                    Command::LRange { list_name, mut start_index, mut end_index, respond_to } => {
                        let entry_ = dict.entry(list_name.clone()).or_insert((RedisList(LinkedList::new()), None));
                        if let Some((RedisList(rl_), _)) = dict.get(&list_name) {
                            let n = rl_.len() as i32;
                            if start_index < -n {start_index = 0;}
                            if end_index < -n {end_index = 0;}
                            if end_index >= n {end_index = n-1;}
                            if start_index >= n {
                                let _ = respond_to.send(b"*0\r\n".to_vec());
                                continue;
                            }
                            start_index = ((start_index % n) + n)%n;
                            end_index = ((end_index % n) + n)%n;
                            println!("{:?}", rl_);
                            if start_index > end_index {
                                let _ = respond_to.send(b"*0\r\n".to_vec());
                                continue;
                            }
                            let mut return_str = Vec::<u8>::new();
                            write!(&mut return_str, "*{}\r\n", end_index - start_index + 1).unwrap();
                            for (ind,it) in rl_.iter().enumerate() {
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
                    Command::LLen { list_name, respond_to } => {
                        if let Some((RedisList(rl_), _)) = dict.get(&list_name) {
                            let _ = respond_to.send(rl_.len());
                        }
                        else {
                            let _ = respond_to.send(0);
                        }

                    },
                    Command::LPOP { list_name, no_of_elements, respond_to } => {
                        if let Some(no_of_elements) = no_of_elements && let Some((RedisList(rl_), _)) = dict.get_mut(&list_name)  {
                            if rl_.len() == 0 {
                                let _ = respond_to.send(b"$-1\r\n".to_vec());
                                continue;
                            }
                            let mut buf = Vec::<u8>::new();
                            let n_ = std::cmp::min(no_of_elements, rl_.len() as i32);
                            write!(&mut buf, "*{}\r\n", n_);
                            for i in 0..n_ {
                                let p = rl_.pop_front().unwrap();
                                let mut tmp_buf = Vec::<u8>::new();
                                write!(&mut tmp_buf, "${}\r\n", p.len());
                                tmp_buf.extend_from_slice(p.as_slice());
                                tmp_buf.extend_from_slice(b"\r\n");
                                buf.extend_from_slice(&tmp_buf);
                            }
                            let _ = respond_to.send(buf);
                        } else if let Some((RedisList(rl_), _)) = dict.get_mut(&list_name) {
                            if rl_.len() == 0 {
                                let _ = respond_to.send(b"$-1\r\n".to_vec());
                                continue;
                            }
                            let p = rl_.pop_front().unwrap();
                            let mut buf = Vec::<u8>::new();
                            write!(&mut buf, "${}\r\n", p.len());
                            buf.extend_from_slice(&p);
                            buf.extend_from_slice(b"\r\n");
                            let _ = respond_to.send(buf);
                        }
                        else {
                            let _ = respond_to.send(b"".to_vec());
                        }
                    },
                    Command::BLPOP { list_name, exp_time, respond_to } => {
                        // println!("laadle");
                        let now_ts = std::time::Instant::now();
                        let fin_time = match exp_time {
                            Some(e_t) => Some(now_ts + e_t),
                            None => None
                        };
                        println!("{:?}", fin_time);
                        waiters.entry(list_name).or_insert(BinaryHeap::new()).push(Waiter{deadline: fin_time, respond_to});
                    },
                    Command::TYPE { key, respond_to } => {
                        match dict.get(&key) {
                            Some((RedisList(_), ts)) if ts.is_none() || *ts >= Some(now_ts) => {let _ = respond_to.send(b"+list\r\n".to_vec());}
                            Some((RedisString(_), ts)) if ts.is_none() || *ts >= Some(now_ts) => {let _ = respond_to.send(b"+string\r\n".to_vec());}
                            Some((RedisStream(_), ts)) if ts.is_none() || *ts >= Some(now_ts) => {let _ = respond_to.send(b"+stream\r\n".to_vec());}

                            _ => {let _ = respond_to.send(b"+none\r\n".to_vec());}
                        }
                    },
                    Command::XADD {key, stream_id, value_pairs, respond_to} => {
                        // if !streams_.search(&key) {
                        //     streams_.insert(&key);
                        // }
                        // let it_ = streams_.iterator(&key).unwrap();
                        match dict.get_mut(&key) {
                            Some((RedisStream(rs_), ts)) if (ts.is_none() | (*ts >= Some(Instant::now())))  => {
                                if !rs_.search(&stream_id) {
                                    rs_.insert(&stream_id);
                                }
                                let it_ = rs_.iterator(&stream_id).unwrap();
                                for x in value_pairs {
                                    it_.insert(x.0, x.1);
                                }
                            },
                            
                            Some((_, ts)) if (ts.is_none() | (*ts >= Some(Instant::now()))) => {
                                let _ = respond_to.send(b"+Error\r\n".to_vec());
                                continue;
                            },
                            _ => {
                                let mut rs_ = radx::RadixT::new();
                                rs_.insert(&stream_id);
                                println!("{:?}", rs_);
                                let it_ = rs_.iterator(&stream_id).unwrap();
                                for x in value_pairs {
                                    it_.insert(x.0, x.1);
                                }
                                dict.insert(key.clone(), (RedisStream(rs_), None));
                            }
                        }
                        let mut buf = Vec::<u8>::new();
                        write!(&mut buf, "${}\r\n", stream_id.len());
                        buf.extend_from_slice(&stream_id);
                        buf.extend_from_slice(b"\r\n");
                        let _ = respond_to.send(buf);
                        continue;
                    }
                }
            },

            _ = tokio::time::sleep(time::Duration::from_millis(10)) => {
                // println!("tttt");
                for x in waiters.iter_mut() {
                    loop {
                        let now_ = Instant::now();

                        let front_element = x.1.peek();
                        if let Some(waiter_) = front_element {
                            if waiter_.deadline < Some(now_) && waiter_.deadline.is_some() {
                                let front_ = x.1.pop().unwrap();
                                let _ = front_.respond_to.send(b"*-1\r\n".to_vec());
                                println!("A")
                            }
                            else if let Some((RedisList(rl_), _)) = dict.get_mut(x.0) && rl_.len() > 0  {
                                let list_name = x.0;
                                let mut buffer = Vec::<u8>::new();

                                write!(&mut buffer, "*2\r\n${}\r\n", list_name.len());
                                buffer.extend_from_slice(&list_name);
                                buffer.extend_from_slice(b"\r\n");
                                let mut tmp_buff = Vec::<u8>::new();
                                write!(&mut tmp_buff, "${}\r\n", rl_.front().unwrap().len());
                                tmp_buff.extend_from_slice(rl_.front().unwrap());
                                tmp_buff.extend_from_slice(b"\r\n");
                                buffer.extend_from_slice(&tmp_buff);

                                rl_.pop_front().unwrap();
                                let front_ = x.1.pop().unwrap();
                                let _ = front_.respond_to.send(buffer);
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
