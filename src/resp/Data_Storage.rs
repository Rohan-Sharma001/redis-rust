use std::collections::*;
#[derive(PartialEq, Eq, Clone)]
pub enum Data_Storage {
    RedisString(Vec<u8>),
    RedisList (LinkedList<Vec<u8>>),
}
