use core::error;
use std::{ collections::*, default, fmt::{ Error, Result }, io::Read, iter::Map };

use crate::command;
#[derive(Default, Debug, PartialEq)]
struct Node {
    children_: BTreeMap<Vec<u8>, Box<Node>>,
    is_end: bool,
    pub kvp_: HashMap<Vec<u8>, Vec<u8>>,
}
#[derive(PartialEq, Debug)]
pub struct RadixT {
    root: Node,
}
impl RadixT {
    pub fn new() -> Self {
        return Self {
            root: Node { ..Default::default() },
        };
    }
    pub fn insert(&mut self, key: &[u8]) {
        self.root.insert(key);
    }
    pub fn search(&self, key: &[u8]) -> bool {
        return self.root.search(key);
    }
    pub fn range(&self, start: &[u8], end: &[u8]) -> Vec<Vec<u8>> {
        return self.root.range(start, end);
    }
    pub fn delete(&mut self, key: &[u8]) -> Result<> {
        return self.root.delete(key);
    }
    pub fn iterator(&mut self, key: &[u8]) -> Option<&mut HashMap<Vec<u8>, Vec<u8>>> {
        return self.root.iterator(key);
    }
}

impl Node {
    fn new() -> Box<Self> {
        return Box::new(Self {
            children_: BTreeMap::new(),
            is_end: false,
            ..Default::default()
        });
    }
    fn iterator(&mut self, key: &[u8]) -> Option<&mut HashMap<Vec<u8>, Vec<u8>>> {
        if key.len() == 0 {
            if self.is_end {
                return Some(&mut self.kvp_)
            }
        }
        for i in 0..key.len()+1 {
            println!("{:?}", self.children_);
            println!("{:?}", &key[0..i]);
            if self.children_.contains_key(&key[0..i]) {
                return self.children_.get_mut(&key[0..i]).unwrap().iterator(&key[i..]);
            }
        }
        return None;
    }
    fn insert(&mut self, key: &[u8]) {
        let mut it_ = &*self;
        let mut ps_ = None;
        let mut l_ = None;
        for x in self.children_.iter_mut() {
            let len_ = common_prefix(x.0, key);
            if len_ == key.len() {
                x.1.is_end = true;
                return;
            } else if len_ == x.0.len() {
                x.1.insert(&key[len_..]);
                return;
            } else if len_ > 0 {
                ps_ = Some(x.0.clone());
                l_ = Some(len_);
                break;
            }
        }
        if let Some(ps_) = ps_ && let Some(l_) = l_ {
            let fp_ = &ps_[0..l_];
            let sp_ = &ps_[l_..];
            let mut y_ = Box::new(Node {
                ..Default::default()
            });

            let x_ = self.children_.remove(&ps_).unwrap();

            y_.children_.insert(sp_.to_vec(), x_);
            y_.children_.insert(
                key[l_..].to_vec(),
                Box::new(Node {
                    is_end: true,
                    ..Default::default()
                })
            );
            self.children_.insert(fp_.to_vec(), y_);
            return;
        }

        self.children_.insert(
            key.to_vec(),
            Box::new(Node {
                is_end: true,
                ..Default::default()
            })
        );
    }
    fn search(&self, key: &[u8]) -> bool {
        if key.len() == 0 {
            return self.is_end;
        }
        for i in 0..key.len() - 1 {
            if self.children_.contains_key(&key[0..i]) {
                return self.search(&key[i..]);
            }
        }
        return false;
    }
    fn range(&self, start: &[u8], end: &[u8]) -> Vec<Vec<u8>> {
        let mut rt_ = Vec::new();
        let mut stack_: VecDeque<(&Node, Vec<u8>)> = VecDeque::new();
        stack_.push_back((self, b"".to_vec()));
        let mut counter_ = -1;
        while let Some((ptr_, prefix_)) = stack_.pop_back() {
            counter_ += 1;
            for x in ptr_.children_.iter().rev() {
                let g_ = [&prefix_[..], x.0].concat();
                if x.1.is_end && g_.as_slice() >= start && g_.as_slice() <= end {
                    rt_.push(g_.clone());
                }
                stack_.push_back((&*x.1, g_));
            }
        }
        rt_
    }
    fn delete(&mut self, key: &[u8]) -> Result<> {
        // println!("{:?}", self);
        if key.len() == 0 {
            if self.is_end {
                self.is_end = false;
                return Ok(());
            } else {
                return Err(Error);
            }
        }
        for i in 0..key.len() + 1 {
            let y = &key[0..i];
            if let Some(x_) = self.children_.get_mut(&key[0..i]) {
                let k = x_.delete(&key[i..]);
                if x_.children_.len() == 0 && x_.is_end == false {
                    self.children_.remove(&key[0..i].to_vec());
                }
                return k;
            }
        }
        return Err(Error);
    }
}
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    if a.len() == 0 || b.len() == 0 {
        return 0;
    }
    let mut it_: usize = 0;
    while let Some(a_) = a.get(it_) && let Some(b_) = b.get(it_) {
        if a_ != b_ {
            break;
        }
        it_ += 1;
    }
    return it_;
}
