use std::collections::*;

use crate::radx::RadixT;
// #[derive(Clone)]
pub enum Data_Storage {
    RedisString(Vec<u8>),
    RedisList (LinkedList<Vec<u8>>),
    RedisStream(RadixT)
}
