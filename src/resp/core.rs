use std::error::Error;

use super::serialize::SerializeRESP;

pub const CLRF: &[u8] = b"\r\n";
pub const NULL_PREFIX: &[u8] = b"_";
pub const NULL_STRING_PREFIX: &[u8] = b"$-1";
pub const NULL_ARRAY_PREFIX: &[u8] = b"*-1";
pub const SIMPLE_STRING_PREFIX: &[u8] = b"+";
pub const SIMPLE_ERROR_PREFIX: &[u8] = b"-";
pub const INTEGER_PREFIX: &[u8] = b":";
pub const DOUBLE_PREFIX: &[u8] = b",";
pub const BOOLEAN_PREFIX: &[u8] = b"#";
pub const BULK_STRING_PREFIX: &[u8] = b"$";
pub const BULK_BUF_STRING_PREFIX: &[u8] = b"$";
pub const ARRAY_PREFIX: &[u8] = b"*";

#[derive(Debug)]
pub enum RESPDatatypes {
    Null,
    NullString,
    NullArray,

    Integer(i32),
    Double(f64),

    SimpleString(String),
    SimpleError(Box<dyn Error>),
    BulkString(String),
    RDBFile(Vec<u8>),
    BufBulk(Vec<u8>),

    Boolean(bool),

    Array(Vec<RESPDatatypes>),
}

impl RESPDatatypes {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        SerializeRESP.encode(self, &mut buf);
        buf
    }
}

unsafe impl Send for RESPDatatypes {}
unsafe impl Sync for RESPDatatypes {}
