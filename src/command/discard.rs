use std::io;

use crate::resp::{core::RESPDatatypes, deserialize::bytes_to_string};

use super::core::Command;

#[derive(Default)]
pub struct Discard;

impl Command for Discard {
    fn can_execute(&mut self, cmd: &RESPDatatypes) -> bool {
        if let RESPDatatypes::Array(vec) = cmd {
            if vec.len() != 1 {
                return false;
            }
            let cmd = vec.first().unwrap();
            match cmd {
                RESPDatatypes::BufBulk(vec)
                    if bytes_to_string(&vec.to_vec())
                        .unwrap_or("".to_string())
                        .to_lowercase()
                        == "discard" =>
                {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
        false
    }

    fn run(
        &mut self,
        _cache_repo: std::sync::Arc<tokio::sync::Mutex<crate::cache::core::CacheRepository>>,
        conn: Option<&mut crate::connections::connection::Connection>,
    ) -> super::core::RunResult {
        if let Some(conn) = conn {
            if conn.is_in_transaction() {
                conn.discard_transaction();
                return Box::pin(async move { Ok(RESPDatatypes::SimpleString("OK".to_string())) });
            }
        }
        Box::pin(async move {
            Ok(RESPDatatypes::SimpleError(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ERR DISCARD without MULTI",
            ))))
        })
    }
}
