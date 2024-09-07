use crate::resp::{core::RESPDatatypes, deserialize::bytes_to_string};

use super::core::Command;

pub struct Psync;

impl Command for Psync {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
        if let RESPDatatypes::Array(vec) = cmd {
            if vec.len() != 3 {
                return false;
            }
            let cmd = vec.first().unwrap();
            match cmd {
                RESPDatatypes::BufBulk(vec)
                    if bytes_to_string(&vec.to_vec()).unwrap_or("".to_string()) == "PSYNC" =>
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
        let conn = conn.unwrap();
        conn.send_rdb_file = Some(());
        let server_replication_config = conn.server_config.replication_config.clone();

        let replication_id = server_replication_config
            .master_repl_id
            .unwrap_or("8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string());
        let replication_offset = server_replication_config.master_repl_offset.unwrap_or(0);

        let message = format!("FULLRESYNC {} {}", replication_id, replication_offset);

        Box::pin(async move { Ok(RESPDatatypes::SimpleString(message)) })
    }
}
