use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::oneshot, sync::mpsc, time::Interval};
pub enum Command {
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
    },
    LLen {
        list_name: Vec<u8>,
        respond_to: oneshot::Sender<usize>
    },
    LPOP {
        list_name: Vec<u8>,
        no_of_elements: Option<i32>,
        respond_to: oneshot::Sender<Vec<u8>>
    }
}
