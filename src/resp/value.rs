use core::fmt;
#[derive(PartialEq)]
pub enum DataObjects {
    BasicString(String),
    Error(String),
    Integer(f64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<DataObjects>>),
}

impl DataObjects {
    pub fn as_command(&self) -> Option<&[u8]> {
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
            DataObjects::BulkString(Some(arr)) =>
                write!(f, "{}", String::from_utf8(arr.clone()).unwrap()),
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
            }
            _ => { Ok(()) }
        }
    }
}
