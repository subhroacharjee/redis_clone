use std::{
    io::{self},
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

use crate::{
    cache::core::CacheRepository,
    cli::config::Config,
    cmd_queue::core::CmdQueue,
    command::core::{run, run_command, Command},
    resp::{
        core::RESPDatatypes,
        deserialize::{bytes_to_string, Deseralize},
    },
};

#[derive(Debug)]
pub struct SlaveConfig {
    pub port: String,
    pub exp: Instant,
    pub last_cmd_id: Option<String>,
    pub send_output: Option<bool>,
    pub bytes_offset: u16,
}

pub struct Connection {
    pub id: String,
    pub stream: TcpStream,
    pub in_transaction: bool,
    pub repo: Arc<Mutex<CacheRepository>>,
    pub tnxs: Option<Vec<Box<dyn Command + 'static>>>,
    pub server_config: Config,
    pub slave_config: Option<SlaveConfig>,
    pub send_rdb_file: Option<()>,
    pub cmdq: Arc<Mutex<CmdQueue>>,
    pub is_master: bool,
}

impl Connection {
    pub fn new(
        stream: TcpStream,
        addr: SocketAddr,
        repo: Arc<Mutex<CacheRepository>>,
        config: Config,
        cmdq: Arc<Mutex<CmdQueue>>,
        is_master: bool,
    ) -> Self {
        Connection {
            id: addr.ip().to_string(),
            stream,
            in_transaction: false,
            tnxs: None,
            repo,
            server_config: config,
            slave_config: None,
            send_rdb_file: None,
            cmdq,
            is_master,
        }
    }

    pub async fn process(&mut self) {
        loop {
            // println!("called with {}", self.send_rdb_file.is_none());
            if self.is_master && self.process_master().await {
                break;
            } else if self.send_rdb_file.is_none() && self.process_client().await {
                break;
            } else if self.send_rdb_file.is_some() && self.process_slave().await {
                break;
            }
        }
    }

    async fn process_master(&mut self) -> bool {
        let repo = self.repo.clone();
        let mut buff: Vec<u8> = Vec::new();

        let mut buf = Vec::with_capacity(10);
        let mut counter = 0;
        loop {
            let size = self
                .stream
                .read_buf(&mut buf)
                .await
                .expect("cant read message from master");

            buff.append(&mut buf);

            if size <= counter || size == 0 {
                break;
            }
            counter = size;

            buf = Vec::with_capacity(100);
        }
        let mut last_rdb_idx = 0;

        loop {
            if last_rdb_idx >= buff.len() {
                break;
            }
            let (_, rest) = buff.split_at(last_rdb_idx);

            match bytes_to_string(rest) {
                Ok(_) => {
                    break;
                }
                Err(err) if err.kind() == io::ErrorKind::InvalidInput => {
                    last_rdb_idx += 1;
                }
                Err(e) => {
                    println!("{}", e);
                    panic!("something went wrong");
                }
            };
        }

        let (_, mut rem_cmd_buf) = buff.split_at(last_rdb_idx);
        let mut cmd_idx = 0;
        let deserialzer = Deseralize {};

        loop {
            let cache_repo = repo.clone();
            let (cmd, rem) = rem_cmd_buf.split_at(cmd_idx);

            match deserialzer.deseralize(&mut cmd.to_vec()) {
                Ok(RESPDatatypes::Array(resp)) => {
                    println!("called for cmd {:?}", resp);
                    let cmd = RESPDatatypes::Array(resp);
                    let byte_size = cmd.encode().len();
                    let op = run_command(cmd, cache_repo, self).await;
                    rem_cmd_buf = rem;
                    cmd_idx = 0;
                    if let Some(slave_config) = self.slave_config.as_mut() {
                        if let Some(send_output) = slave_config.send_output.take() {
                            if send_output {
                                self.stream.write_all(&op).await.unwrap();
                            }
                        }

                        slave_config.bytes_offset += byte_size as u16;
                        println!("{:?}", slave_config);
                    }

                    continue;
                }

                _ => {
                    // println!("{:?}", bytes_to_string(cmd));
                    cmd_idx += 1;
                }
            }

            if rem.is_empty() {
                break;
            }
        }
        false
    }

    async fn process_client(&mut self) -> bool {
        let repo = self.repo.clone();
        let mut buff = Vec::new();

        let read_count = self.stream.read_buf(&mut buff).await.unwrap_or(0);
        if read_count == 0 {
            return true;
        }

        let res = run(&mut buff, repo, self).await;
        if res.is_empty() {
            println!("found eof");
            return true;
        }

        if !self.is_master {
            self.stream.write_all(&res).await.unwrap();
            self.send_rdb_file_to_replica().await;
        }
        false
    }

    async fn process_slave(&mut self) -> bool {
        let mut slave_config = self.slave_config.take().unwrap();
        let last_cmd_id = slave_config.last_cmd_id.take();
        let cmdq = self.cmdq.lock().await;
        if let Some((last_id, mut cmd_buffs)) = cmdq.get_all_cmds_after_id(last_cmd_id).await {
            slave_config.last_cmd_id = Some(last_id);
            while let Some(cmd) = cmd_buffs.pop() {
                match self.stream.write_all(&cmd).await {
                    Ok(_) => {}
                    Err(err) => {
                        println!("slave has died err {}", err);
                        return true;
                    }
                }
            }
        }

        self.slave_config.replace(slave_config);
        false
    }

    pub fn is_in_transaction(&self) -> bool {
        self.in_transaction
    }

    pub fn enable_transaction(&mut self) {
        self.in_transaction = true;
        self.tnxs = Some(Vec::new());
    }

    pub fn discard_transaction(&mut self) {
        self.in_transaction = false;
        if let Some(mut tnxs) = self.tnxs.take() {
            tnxs.clear();
        }
    }

    pub fn add_tnx(&mut self, cmd: Box<dyn Command>) {
        if let Some(tnxs) = self.tnxs.as_mut() {
            tnxs.push(cmd);
        }
    }

    pub fn get_tnxs(&mut self) -> Option<Vec<Box<dyn Command>>> {
        self.tnxs.take()
    }

    pub fn get_id(&self) -> String {
        self.id.to_string()
    }

    pub async fn send_rdb_file_to_replica(&mut self) {
        // println!("{:?}", self.send_rdb_file);
        if self.send_rdb_file.is_some() {
            // println!("called for send_rdb_file");

            let res = self.send_empty_rdb_file();
            let buff = match res {
                Ok(buf) => buf,
                Err(err) => {
                    self.slave_config.take();
                    RESPDatatypes::SimpleError(Box::new(io::Error::new(
                        io::ErrorKind::InvalidData,
                        err,
                    )))
                    .encode()
                }
            };

            self.stream.write_all(&buff).await.unwrap();
        }
    }

    pub fn send_empty_rdb_file(&mut self) -> io::Result<Vec<u8>> {
        let vec = hex::decode("524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2").unwrap();
        Ok(RESPDatatypes::RDBFile(vec).encode())
    }
}
