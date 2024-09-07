use std::io;

use crate::{
    cli::config::ToConfigString,
    resp::{core::RESPDatatypes, deserialize::bytes_to_string},
};

use super::core::Command;

#[derive(Debug, Default)]
pub struct Info {
    sub_command: String,
}

impl Command for Info {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
        if let RESPDatatypes::Array(vecs) = cmd {
            if vecs.len() != 2 {
                return false;
            }

            let first_elem = vecs.first().unwrap();
            match first_elem {
                RESPDatatypes::BufBulk(first_elem)
                    if bytes_to_string(first_elem)
                        .unwrap_or("".to_string())
                        .to_lowercase()
                        == "info" =>
                {
                    if let RESPDatatypes::BufBulk(second_elem) = vecs.get(1).unwrap() {
                        self.sub_command = bytes_to_string(second_elem)
                            .unwrap_or("".to_string())
                            .to_lowercase();
                    }
                    return true;
                }
                _ => {}
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
            let server_config = conn.server_config.clone();

            return Box::pin(async move {
                if self.sub_command == "replication" {
                    return Ok(RESPDatatypes::BulkString(
                        server_config.replication_config.to_config_string(),
                    ));
                }
                Ok(RESPDatatypes::SimpleString("OK".to_string()))
            });
        }
        Box::pin(async move {
            Ok(RESPDatatypes::SimpleError(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ERR EXEC without MULTI",
            ))))
        })
    }
}
