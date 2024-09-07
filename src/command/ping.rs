use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    cache::core::CacheRepository,
    connections::connection::Connection,
    resp::{core::RESPDatatypes, deserialize::bytes_to_string},
};

use super::core::{Command, RunResult};

#[derive(Debug, Default)]
pub struct Ping {}

impl Ping {
    pub fn get_output(&mut self) -> RESPDatatypes {
        RESPDatatypes::SimpleString("PONG".to_string())
    }
}

impl Command for Ping {
    fn can_execute(&mut self, raw_cmd: &RESPDatatypes) -> bool {
        match raw_cmd {
            RESPDatatypes::SimpleString(val) => val.to_lowercase().starts_with("ping"),
            RESPDatatypes::BufBulk(val) => bytes_to_string(val)
                .unwrap_or("".to_string())
                .to_lowercase()
                .starts_with("ping"),

            RESPDatatypes::Array(arr_of_resp_data) => {
                if arr_of_resp_data.is_empty() {
                    return false;
                }

                if let Some(first_elem) = arr_of_resp_data.first() {
                    return self.can_execute(first_elem);
                }
                false
            }
            _ => false,
        }
    }

    fn run(
        &mut self,
        _cache_repo: Arc<Mutex<CacheRepository>>,
        _conn: Option<&mut Connection>,
    ) -> RunResult {
        Box::pin(async move { Ok(self.get_output()) })
    }
}
