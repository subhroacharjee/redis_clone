use std::time::Instant;

use crate::{
    connections::connection::SlaveConfig,
    resp::{core::RESPDatatypes, deserialize::bytes_to_string},
};

use super::core::Command;

#[derive(Default)]
pub struct ReplConf {
    pub conf_type: String,
    pub conf_data: String,
}

impl Command for ReplConf {
    fn can_execute(&mut self, cmd: &crate::resp::core::RESPDatatypes) -> bool {
        if let RESPDatatypes::Array(vec) = cmd {
            if vec.len() != 3 {
                return false;
            }
            let cmd = vec.first().unwrap();
            if let RESPDatatypes::BufBulk(buff_cmd) = cmd {
                if bytes_to_string(buff_cmd).unwrap_or("".to_string()) != "REPLCONF" {
                    return false;
                }

                let second_elem = vec.get(1).unwrap();
                if let RESPDatatypes::BufBulk(conf_type_buf) = second_elem {
                    self.conf_type = bytes_to_string(conf_type_buf).unwrap_or("".to_string());
                } else {
                    return false;
                }

                let third_elem = vec.get(2).unwrap();
                if let RESPDatatypes::BufBulk(conf_data_buf) = third_elem {
                    self.conf_data = bytes_to_string(conf_data_buf).unwrap_or("".to_string());
                } else {
                    return false;
                }
            }
        }

        true
    }

    fn run(
        &mut self,
        _cache_repo: std::sync::Arc<tokio::sync::Mutex<crate::cache::core::CacheRepository>>,
        conn: Option<&mut crate::connections::connection::Connection>,
    ) -> super::core::RunResult {
        if self.conf_type != "listening-port" && self.conf_type != "GETACK" {
            if let Some(conn) = conn {
                conn.slave_config.replace(SlaveConfig {
                    port: self.conf_data.to_string(),
                    exp: Instant::now(),
                    last_cmd_id: None,
                    send_output: None,
                    bytes_offset: 0,
                });
            }
        } else if self.conf_type == "GETACK" {
            println!("should be callled");
            let mut bytes_size = 0;
            if let Some(conn) = conn {
                if let Some(slave_config) = conn.slave_config.as_mut() {
                    slave_config.send_output.replace(true);
                    bytes_size = slave_config.bytes_offset;
                } else {
                    conn.slave_config.replace(SlaveConfig {
                        port: "4000".to_string(),
                        exp: Instant::now(),
                        last_cmd_id: None,
                        send_output: Some(true),
                        bytes_offset: 0,
                    });
                }
            }
            return Box::pin(async move {
                Ok(RESPDatatypes::Array(vec![
                    RESPDatatypes::BulkString("REPLCONF".to_string()),
                    RESPDatatypes::BulkString("ACK".to_string()),
                    RESPDatatypes::BulkString(format!("{}", bytes_size)),
                ]))
            });
        }

        Box::pin(async move { Ok(RESPDatatypes::SimpleString("OK".to_string())) })
    }
}
