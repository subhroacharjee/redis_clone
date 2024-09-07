use std::{io, time::Duration};

use crate::resp::{core::RESPDatatypes, deserialize::bytes_to_string};

use super::core::Command;

#[derive(Default)]
pub struct Exec;

impl Command for Exec {
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
                        == "exec" =>
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
        cache_repo: std::sync::Arc<tokio::sync::Mutex<crate::cache::core::CacheRepository>>,
        conn: Option<&mut crate::connections::connection::Connection>,
    ) -> super::core::RunResult {
        if let Some(conn) = conn {
            if conn.is_in_transaction() {
                let tnx_id = conn.get_id().to_string();
                let tnxs = conn.get_tnxs();
                conn.discard_transaction();
                return Box::pin(async move {
                    let repo = cache_repo.clone();
                    let mut tnxs = tnxs.unwrap_or(vec![]);
                    repo.lock().await.set_transaction(tnx_id.to_string()).await;
                    let handler_repo = repo.clone();
                    let cache = repo.clone();

                    let handler = tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        match { handler_repo.lock().await.get_transaction_id().await } {
                            Some(curr_id) if curr_id == tnx_id.to_string() => {
                                handler_repo.lock().await.unset_transaction().await;
                            }
                            _ => {}
                        }
                    });
                    let mut resp = vec![];
                    for cmd in tnxs.iter_mut() {
                        let cache_repo = cache.clone();
                        resp.push(
                            cmd.run(cache_repo, None)
                                .await
                                .unwrap_or_else(|err| RESPDatatypes::SimpleError(Box::new(err))),
                        );
                    }
                    cache.lock().await.unset_transaction().await;

                    handler.abort_handle().abort();
                    Ok(RESPDatatypes::Array(resp))
                });
            }
        }
        Box::pin(async move {
            Ok(RESPDatatypes::SimpleError(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ERR EXEC without MULTI",
            ))))
        })
    }
}
