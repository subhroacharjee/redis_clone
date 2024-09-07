use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    cache::core::CacheRepository,
    connections::connection::Connection,
    resp::{core::RESPDatatypes, deserialize::bytes_to_string},
};

use super::core::{Command, RunResult};

#[derive(Debug, Default)]
pub struct Echo {
    pub data: Option<Vec<u8>>,
}

impl Echo {}

impl Command for Echo {
    fn can_execute(&mut self, raw_cmd: &RESPDatatypes) -> bool {
        if let RESPDatatypes::Array(vec) = raw_cmd {
            if vec.is_empty() {
                return false;
            }
            if let Some(RESPDatatypes::BufBulk(buf)) = vec.first() {
                if bytes_to_string(buf)
                    .unwrap_or("".to_string())
                    .to_lowercase()
                    != "echo"
                {
                    return false;
                }

                if let Some(RESPDatatypes::BufBulk(buf)) = vec.get(1) {
                    self.data = Some(buf.to_vec());
                }
            }
        }
        true
    }

    fn run(
        &mut self,
        _cache_repo: Arc<Mutex<CacheRepository>>,
        _conn: Option<&mut Connection>,
    ) -> RunResult {
        Box::pin(async move {
            if let Some(buf) = self.data.as_ref() {
                return Ok(RESPDatatypes::BufBulk(buf.to_vec()));
            }
            Ok(RESPDatatypes::NullString)
        })
    }
}
