use std::io::{self, Error};

use crate::{
    connections::connection::Connection,
    errors::value_is_not_type::ValueIsNotType,
    resp::{
        core::RESPDatatypes,
        deserialize::{bytes_to_string, bytes_to_type},
    },
};

use super::core::{Command, RunResult};

#[derive(Debug, Default)]
pub struct Incr {
    pub cmd: Vec<u8>,
    pub key: String,
}

impl Incr {
    fn new(key: String) -> Self {
        Incr { key, cmd: vec![] }
    }
}

impl Command for Incr {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
        if let RESPDatatypes::Array(arr) = cmd {
            if arr.len() < 2 {
                return false;
            }

            let first_elem = arr.first().unwrap();
            if let RESPDatatypes::BufBulk(buf) = first_elem {
                if bytes_to_string(buf)
                    .unwrap_or("".to_string())
                    .to_lowercase()
                    == "incr"
                {
                    if let RESPDatatypes::BufBulk(scnd_elem) = arr.get(1).unwrap() {
                        self.key = bytes_to_string(scnd_elem).unwrap_or("".to_string());
                        self.cmd = cmd.encode();
                        return true;
                    }
                    return false;
                }
            } else {
                return false;
            }
        }
        false
    }

    // this implementation is wrong i need to pass only the result;
    fn run(
        &mut self,
        cache_repo: std::sync::Arc<tokio::sync::Mutex<crate::cache::core::CacheRepository>>,
        conn: Option<&mut Connection>,
    ) -> RunResult {
        match conn {
            Some(conn) => {
                if conn.is_in_transaction() {
                    conn.add_tnx(Box::new(Self::new(self.key.to_string())));
                    return Box::pin(async move {
                        Ok(RESPDatatypes::SimpleString("QUEUED".to_string()))
                    });
                }
                let cmdq = conn.cmdq.clone();
                let cmd = self.cmd.to_vec();
                let jh = tokio::spawn(async move {
                    let mut cmd_queue = cmdq.lock().await;
                    cmd_queue.add(cmd).await;
                });

                Box::pin(async move {
                    let cache = cache_repo.clone();
                    let key = self.key.to_string();
                    let mut val = 1;

                    if let Some(existing_data) = cache.lock().await.get(key.to_string()).await {
                        match bytes_to_type::<i32>(&existing_data.to_vec()) {
                            Ok(existing_val) => {
                                val = existing_val + 1;
                            }
                            Err(err) => {
                                println!("err: {}", err);
                                return Err(Error::new(
                                    io::ErrorKind::InvalidInput,
                                    ValueIsNotType {
                                        type_name: "integer".to_string(),
                                        can_be_out_of_range: Some(true),
                                    },
                                ));
                            }
                        }
                    }

                    let value = format!("{}", val).into_bytes();
                    cache.lock().await.set(key, value).await?;
                    jh.await?;
                    Ok(RESPDatatypes::Integer(val))
                })
            }
            None => Box::pin(async move {
                let cache = cache_repo.clone();
                let key = self.key.to_string();
                let mut val = 1;

                if let Some(existing_data) = cache.lock().await.get(key.to_string()).await {
                    match bytes_to_type::<i32>(&existing_data.to_vec()) {
                        Ok(existing_val) => {
                            val = existing_val + 1;
                        }
                        Err(err) => {
                            println!("err: {}", err);
                            return Err(Error::new(
                                io::ErrorKind::InvalidInput,
                                ValueIsNotType {
                                    type_name: "integer".to_string(),
                                    can_be_out_of_range: Some(true),
                                },
                            ));
                        }
                    }
                }

                let value = format!("{}", val).into_bytes();
                cache.lock().await.set(key, value).await?;
                Ok(RESPDatatypes::Integer(val))
            }),
        }
    }
}
