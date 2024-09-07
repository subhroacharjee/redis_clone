use crate::resp::{core::RESPDatatypes, deserialize::bytes_to_string};

use super::core::Command;

#[derive(Default)]
pub struct Multi;

impl Command for Multi {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
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
                        == "multi" =>
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
            conn.enable_transaction();
        }

        Box::pin(async move { Ok(RESPDatatypes::SimpleString("OK".to_string())) })
    }
}
