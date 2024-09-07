use std::error::Error;

use super::core::{
    RESPDatatypes, ARRAY_PREFIX, BOOLEAN_PREFIX, BULK_STRING_PREFIX, CLRF, DOUBLE_PREFIX,
    INTEGER_PREFIX, NULL_ARRAY_PREFIX, NULL_PREFIX, NULL_STRING_PREFIX, SIMPLE_ERROR_PREFIX,
    SIMPLE_STRING_PREFIX,
};

pub struct SerializeRESP;

impl SerializeRESP {
    pub fn encode(&self, value: &RESPDatatypes, buf: &mut Vec<u8>) {
        match value {
            RESPDatatypes::Null => self.encode_null(buf),
            RESPDatatypes::NullString => self.encode_null_string(buf),
            RESPDatatypes::NullArray => self.encode_null_array(buf),
            RESPDatatypes::Integer(data) => self.encode_integer(buf, data),
            RESPDatatypes::Double(data) => self.encode_double(buf, data),
            RESPDatatypes::SimpleString(data) => self.encode_simple_string(buf, data),
            RESPDatatypes::SimpleError(data) => self.encode_error_string(buf, data),
            RESPDatatypes::BulkString(data) => self.encode_bulk_string(buf, data),
            RESPDatatypes::BufBulk(data) => self.encode_buf_string(buf, data, true),
            RESPDatatypes::Boolean(data) => self.encode_boolean(buf, data),
            RESPDatatypes::Array(data) => self.encode_array(buf, data),
            RESPDatatypes::RDBFile(data) => self.encode_buf_string(buf, data, false),
        };
    }

    fn encode_null(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(NULL_PREFIX);
        buf.extend_from_slice(CLRF);
    }

    fn encode_null_string(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(NULL_STRING_PREFIX);
        buf.extend_from_slice(CLRF);
    }

    fn encode_null_array(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(NULL_ARRAY_PREFIX);
        buf.extend_from_slice(CLRF);
    }

    fn encode_integer(&self, buf: &mut Vec<u8>, data: &i32) {
        buf.extend_from_slice(INTEGER_PREFIX);
        buf.extend_from_slice(&data.to_string().into_bytes());
        buf.extend_from_slice(CLRF);
    }

    fn encode_double(&self, buf: &mut Vec<u8>, data: &f64) {
        buf.extend_from_slice(DOUBLE_PREFIX);
        buf.extend_from_slice(data.to_string().as_bytes());
        buf.extend_from_slice(CLRF);
    }

    fn encode_simple_string(&self, buf: &mut Vec<u8>, data: &String) {
        buf.extend_from_slice(SIMPLE_STRING_PREFIX);
        buf.extend_from_slice(&data.to_string().into_bytes());
        buf.extend_from_slice(CLRF);
    }

    fn encode_error_string(&self, buf: &mut Vec<u8>, data: &Box<dyn Error>) {
        buf.extend_from_slice(SIMPLE_ERROR_PREFIX);
        buf.extend_from_slice(&format!("{}", data).into_bytes());
        buf.extend_from_slice(CLRF);
    }

    fn encode_bulk_string(&self, buf: &mut Vec<u8>, data: &String) {
        buf.extend_from_slice(BULK_STRING_PREFIX);
        buf.extend_from_slice(&format!("{}", data.len()).into_bytes());
        buf.extend_from_slice(CLRF);
        buf.extend_from_slice(&data.to_string().into_bytes());
        buf.extend_from_slice(CLRF);
    }

    fn encode_buf_string(&self, buf: &mut Vec<u8>, data: &Vec<u8>, add_clrf_to_end: bool) {
        buf.extend_from_slice(BULK_STRING_PREFIX);
        buf.extend_from_slice(&format!("{}", data.len()).into_bytes());
        buf.extend_from_slice(CLRF);
        buf.extend_from_slice(data);
        if add_clrf_to_end {
            buf.extend_from_slice(CLRF);
        }
    }

    fn encode_boolean(&self, buf: &mut Vec<u8>, data: &bool) {
        buf.extend_from_slice(BOOLEAN_PREFIX);
        buf.extend_from_slice(&format!("{}", data).into_bytes());
        buf.extend_from_slice(CLRF);
    }

    fn encode_array(&self, buf: &mut Vec<u8>, data: &Vec<RESPDatatypes>) {
        buf.extend_from_slice(ARRAY_PREFIX);
        buf.extend_from_slice(&format!("{}", data.len()).into_bytes());
        buf.extend_from_slice(CLRF);
        for item in data {
            self.encode(item, buf);
        }
    }
}
