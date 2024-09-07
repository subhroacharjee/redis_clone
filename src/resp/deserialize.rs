use std::io::{self, Error, Result};

use crate::errors::{command_not_found::CommandNotFoundError, eof::Eof};

use super::core::{RESPDatatypes, CLRF};

pub struct Deseralize;

pub fn bytes_to_string(vec: &[u8]) -> Result<String> {
    match String::from_utf8(vec.to_vec()) {
        Ok(str) => Ok(str),
        Err(err) => Err(Error::new(io::ErrorKind::InvalidInput, err)),
    }
}

pub fn bytes_to_type<T: std::str::FromStr>(vec: &[u8]) -> Result<T>
where
    <T as std::str::FromStr>::Err: std::convert::Into<
        std::boxed::Box<(dyn std::error::Error + std::marker::Send + std::marker::Sync + 'static)>,
    >,
{
    let str_val = bytes_to_string(vec)?;
    match str_val.trim().parse() {
        Ok(v) => Ok(v),
        Err(err) => Err(Error::new(io::ErrorKind::InvalidInput, err)),
    }
}

pub fn index_till_first_clrf(vec: &mut Vec<u8>) -> usize {
    let mut index = 0;
    for buff in vec.windows(2) {
        if buff == CLRF {
            break;
        }
        index += 1;
    }
    index
}

pub fn drain_till_index_till_first_clrf(vec: &mut Vec<u8>) {
    let mut index = 0;
    for buff in vec.windows(2) {
        if buff == CLRF {
            break;
        }
        index += 1;
    }
    vec.drain(0..index + 2);
}

impl Deseralize {
    pub fn deseralize(&self, input: &mut Vec<u8>) -> Result<RESPDatatypes> {
        let len = input.len();
        if len == 0 {
            return Err(Error::new(io::ErrorKind::UnexpectedEof, Eof {}));
        }
        if len < 3 {
            return Err(Error::new(
                io::ErrorKind::InvalidInput,
                CommandNotFoundError {
                    cmd: String::from_utf8_lossy(input).to_string(),
                },
            ));
        }
        let last_two_bytes = &input[(input.len() - 2)..];
        if last_two_bytes != CLRF {
            return Err(Error::new(
                io::ErrorKind::InvalidInput,
                CommandNotFoundError {
                    cmd: String::from_utf8_lossy(input).to_string(),
                },
            ));
        }

        let base_type = &[input[0]; 1];

        match base_type {
            b"+" => {
                let idx = index_till_first_clrf(input);
                let data = bytes_to_string(&input[1..idx])?;
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::SimpleString(data))
            }
            b"-" => {
                let idx = index_till_first_clrf(input);
                let data = bytes_to_string(&input[1..idx])?;
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::SimpleError(Box::new(Error::new(
                    io::ErrorKind::Other,
                    data,
                ))))
            }
            b"_" => {
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::Null)
            }
            b":" => {
                let idx = index_till_first_clrf(input);
                let data = bytes_to_type(&input[1..idx])?;
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::Integer(data))
            }
            b"," => {
                let idx = index_till_first_clrf(input);
                let data = bytes_to_type(&input[1..idx])?;
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::Double(data))
            }
            b"#" => {
                let idx = index_till_first_clrf(input);
                let data = bytes_to_type(&input[1..idx])?;
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::Boolean(data))
            }

            b"$" => {
                if input.len() < 6 {
                    return Err(Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid buf string",
                    ));
                }
                let mut index = index_till_first_clrf(input);
                if index > input.len() {
                    return Err(Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid buf string",
                    ));
                }

                let input_buf_len: i32 = bytes_to_type(&input[1..index])?;
                if input_buf_len == -1 {
                    return Ok(RESPDatatypes::NullString);
                }

                if input_buf_len < -1 {
                    return Err(Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid buf string",
                    ));
                }
                drain_till_index_till_first_clrf(input);
                index = index_till_first_clrf(input);
                let data = input[0..index].to_vec();
                drain_till_index_till_first_clrf(input);
                Ok(RESPDatatypes::BufBulk(data))
            }
            b"*" => {
                if input.len() < 6 {
                    return Err(Error::new(io::ErrorKind::InvalidInput, "invalid array"));
                }
                let index = index_till_first_clrf(input);
                if index > input.len() {
                    return Err(Error::new(io::ErrorKind::InvalidInput, "invalid array"));
                }
                let input_buf_len: i32 = bytes_to_type(&input[1..index])?;
                if input_buf_len == -1 {
                    return Ok(RESPDatatypes::NullArray);
                }
                drain_till_index_till_first_clrf(input);

                let mut res: Vec<RESPDatatypes> = Vec::with_capacity(input_buf_len as usize);
                let mut i = 0;
                while i < input_buf_len {
                    match self.deseralize(input) {
                        Ok(data) => {
                            res.push(data);
                        }
                        Err(err) => {
                            println!("error while parsing array input {}", err);
                            return Err(Error::new(io::ErrorKind::InvalidInput, "invalid input"));
                        }
                    }

                    i += 1;
                }

                Ok(RESPDatatypes::Array(res))
            }
            _ => Ok(RESPDatatypes::SimpleString(bytes_to_string(input)?)),
        }
    }
}
