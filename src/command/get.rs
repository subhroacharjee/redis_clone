use crate::{
    connections::connection::Connection,
    resp::{core::RESPDatatypes, deserialize::bytes_to_string},
};

use super::core::{Command, RunResult};

#[derive(Default, Debug)]
pub struct Get {
    pub key: String,
}

impl Get {
    fn new(key: String) -> Self {
        Get { key }
    }

    pub fn get_key(&self) -> String {
        self.key.to_string()
    }

    fn is_get_cmd(&self, vec: &Vec<RESPDatatypes>) -> bool {
        if let Some(first_elem) = vec.first() {
            return match first_elem {
                RESPDatatypes::BufBulk(buff) => {
                    bytes_to_string(buff)
                        .unwrap_or("".to_string())
                        .to_lowercase()
                        == "get"
                }
                _ => false,
            };
        }
        false
    }

    fn set_key_if_possible(&mut self, vec: &Vec<RESPDatatypes>) -> bool {
        if let Some(first_elem) = vec.get(1) {
            return match first_elem {
                RESPDatatypes::BufBulk(buff) => {
                    let key = bytes_to_string(buff).unwrap_or("".to_string());
                    if key.is_empty() {
                        return false;
                    }

                    self.key = key;
                    return true;
                }
                _ => false,
            };
        }
        false
    }
}

impl Command for Get {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
        match cmd {
            RESPDatatypes::Array(vec) => {
                if vec.len() < 2 {
                    return false;
                }

                if !self.is_get_cmd(vec) {
                    return false;
                }

                if !self.set_key_if_possible(vec) {
                    return false;
                }

                true
            }
            _ => false,
        }
    }

    fn run(
        &mut self,
        cache_repo: std::sync::Arc<tokio::sync::Mutex<crate::cache::core::CacheRepository>>,
        conn: Option<&mut Connection>,
    ) -> RunResult {
        match conn {
            Some(conn) if conn.is_in_transaction() => {
                conn.add_tnx(Box::new(Self::new(self.get_key())));
                Box::pin(async move { Ok(RESPDatatypes::SimpleString("QUEUED".to_string())) })
            }
            _ => Box::pin(async move {
                let cache = cache_repo.clone();

                if let Some(data) = cache.lock().await.get(self.get_key()).await {
                    return Ok(RESPDatatypes::BufBulk(data));
                }
                Ok(RESPDatatypes::NullString)
            }),
        }
    }
}
