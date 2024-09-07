use tokio::sync::Mutex;

use crate::{
    connections::connection::Connection,
    resp::{
        core::RESPDatatypes,
        deserialize::{bytes_to_string, bytes_to_type},
    },
};

use super::core::{Command, RunResult};

#[derive(Debug, Default)]
pub struct Set {
    pub cmd: Vec<u8>,
    pub key: String,
    pub value: Vec<u8>,
    pub expiry_ttl: Option<u64>,
}

impl Set {
    fn new(key: String, value: Vec<u8>, expiry_ttl: Option<u64>) -> Self {
        Set {
            key,
            value,
            expiry_ttl,
            cmd: vec![],
        }
    }
    fn get_key(&self) -> String {
        self.key.to_string()
    }

    fn get_value(&self) -> Vec<u8> {
        self.value.to_vec()
    }

    fn is_first_elem_is_set_cmd(&mut self, vec: &Vec<RESPDatatypes>) -> bool {
        if let Some(first_elem) = vec.first() {
            return match first_elem {
                RESPDatatypes::BufBulk(buff) => {
                    bytes_to_string(buff)
                        .unwrap_or("".to_string())
                        .to_lowercase()
                        == "set"
                }
                _ => false,
            };
        }
        false
    }

    fn set_key_when_parsable(&mut self, vec: &Vec<RESPDatatypes>) -> bool {
        if let Some(first_elem) = vec.get(1) {
            return match first_elem {
                RESPDatatypes::BufBulk(buff) => {
                    let key = bytes_to_string(buff).unwrap_or("".to_string());
                    if !key.is_empty() {
                        self.key = key;
                        return true;
                    }
                    false
                }
                _ => false,
            };
        }
        false
    }

    fn set_value_if_buffer(&mut self, vec: &Vec<RESPDatatypes>) -> bool {
        if let Some(second_elem) = vec.get(2) {
            return match second_elem {
                RESPDatatypes::BufBulk(buff) => {
                    self.value = buff.to_vec();
                    true
                }
                _ => false,
            };
        }
        false
    }

    fn set_ttl_if_provided(&mut self, vec: &Vec<RESPDatatypes>) {
        if vec.len() < 5 {
            return;
        }

        if let RESPDatatypes::BufBulk(buff) = vec.get(3).unwrap() {
            let key = bytes_to_string(buff)
                .unwrap_or("".to_string())
                .to_lowercase();
            if key != "px" {
                return;
            }

            if let RESPDatatypes::BufBulk(buff) = vec.get(4).unwrap() {
                let expiry_ttl: u64 = bytes_to_type(buff).unwrap_or(0);
                // println!("{}", expiry_ttl);

                if expiry_ttl == 0 {
                    return;
                }

                self.expiry_ttl = Some(expiry_ttl);
            }
        }
    }
}

impl Command for Set {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
        match cmd {
            RESPDatatypes::Array(vec) => {
                if vec.len() < 3 {
                    return false;
                }
                if !self.is_first_elem_is_set_cmd(vec) {
                    return false;
                }

                if !self.set_key_when_parsable(vec) {
                    return false;
                }

                if !self.set_value_if_buffer(vec) {
                    return false;
                }

                self.set_ttl_if_provided(vec);

                self.cmd = cmd.encode();
                true
            }
            _ => false,
        }
    }

    fn run(
        &mut self,
        cache_repo: std::sync::Arc<Mutex<crate::cache::core::CacheRepository>>,
        conn: Option<&mut Connection>,
    ) -> RunResult {
        match conn {
            Some(conn) => {
                if conn.is_in_transaction() {
                    let key = self.get_key();
                    let value = self.get_value();
                    let expiry_ttl = self.expiry_ttl.take();
                    conn.add_tnx(Box::new(Set::new(key, value, expiry_ttl)));
                    return Box::pin(async move {
                        Ok(RESPDatatypes::SimpleString("QUEUED".to_string()))
                    });
                }

                let cmdq = conn.cmdq.clone();
                let cmd = self.cmd.to_vec();
                tokio::spawn(async move {
                    let mut cmd_queue = cmdq.lock().await;
                    cmd_queue.add(cmd).await;
                });
                Box::pin(async move {
                    let cache_repo_clone = cache_repo.clone();
                    let key = self.get_key();
                    let buff = self.get_value();

                    let mut repo = cache_repo_clone.lock().await;
                    if let Some(ttl) = self.expiry_ttl {
                        repo.set_with_expiry(key, buff, ttl).await?;
                    } else {
                        repo.set(key, buff).await?;
                    }
                    Ok(RESPDatatypes::SimpleString("OK".to_string()))
                })
            }
            None => Box::pin(async move {
                let cache_repo_clone = cache_repo.clone();
                let key = self.get_key();
                let buff = self.get_value();

                let mut repo = cache_repo_clone.lock().await;
                if let Some(ttl) = self.expiry_ttl {
                    repo.set_with_expiry(key, buff, ttl).await?;
                } else {
                    repo.set(key, buff).await?;
                }
                Ok(RESPDatatypes::SimpleString("OK".to_string()))
            }),
        }
    }
}
