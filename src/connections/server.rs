use core::panic;
use std::{
    io::{self, Write},
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use clap::Parser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    task::JoinHandle,
    time,
};

use crate::{
    cache::core::CacheRepository,
    cli::{config::Config, core::BaseCliArgs},
    cmd_queue::core::CmdQueue,
    resp::{core::RESPDatatypes, deserialize::bytes_to_string},
};

use super::connection::Connection;

pub struct Server {
    listener: TcpListener,
    config: Config,
}

pub enum Capabilities {
    Psync,
}

impl Capabilities {
    fn to_string(&self) -> String {
        match self {
            Self::Psync => "psync".to_string(),
        }
    }
}

pub enum Replconf {
    ListeningPort(u16),
    Capa(Capabilities),
}

impl Server {
    pub async fn new() -> Self {
        let args = BaseCliArgs::parse();
        let port = args.get_port_or_default(6379);
        let addr = format!("127.0.0.1:{}", port);

        Server {
            listener: TcpListener::bind(addr).await.unwrap(),
            config: Config {
                server_config: crate::cli::config::ServerConfig { port },
                replication_config: crate::cli::config::ReplicationConfig {
                    role: args.get_role(),
                    master_repl_offset: None,
                    master_repl_id: None,
                },
            },
        }
    }

    pub async fn initalize(&mut self) -> io::Result<Option<TcpStream>> {
        if let crate::cli::core::Roles::Slave(raw_addr) =
            self.config.replication_config.role.clone()
        {
            let addr = raw_addr.trim().replace(" ", ":");

            // we dont want propapate error upwards when server addr is incorrect or something.
            let mut master = TcpStream::connect(addr).await?;
            let ping_cmd =
                RESPDatatypes::Array(vec![RESPDatatypes::BulkString("PING".to_string())]).encode();
            let write_size = master.write(&ping_cmd).await.unwrap();
            if write_size == 0 {
                panic!("unable to PING master");
            }

            let mut buff = Vec::new();
            let read_count = master.read_buf(&mut buff).await.unwrap_or(0);
            if read_count == 0 {
                panic!("Master closed connection")
            }

            let reply = bytes_to_string(&buff)
                .unwrap_or("".to_string())
                .trim()
                .to_string();
            if reply != "+PONG" {
                panic!("Master closed connection")
            }

            self.send_replconf(
                &mut master,
                Replconf::ListeningPort(self.config.server_config.port),
            )
            .await?;

            self.send_replconf(&mut master, Replconf::Capa(Capabilities::Psync))
                .await?;

            self.send_psync(&mut master).await?;

            return Ok(Some(master));
        }
        Ok(None)
    }

    pub async fn send_replconf(&mut self, master: &mut TcpStream, arg: Replconf) -> io::Result<()> {
        let cmd = match arg {
            Replconf::ListeningPort(port) => RESPDatatypes::Array(vec![
                RESPDatatypes::BulkString("REPLCONF".to_string()),
                RESPDatatypes::BulkString("listening-port".to_string()),
                RESPDatatypes::BulkString(format!("{}", port)),
            ])
            .encode(),
            Replconf::Capa(cap) => RESPDatatypes::Array(vec![
                RESPDatatypes::BulkString("REPLCONF".to_string()),
                RESPDatatypes::BulkString("capa".to_string()),
                RESPDatatypes::BulkString(cap.to_string()),
            ])
            .encode(),
        };

        let write_size = master.write(&cmd).await.unwrap_or(0);
        if write_size == 0 {
            panic!("unable to PING master");
        }

        let mut buff = Vec::new();
        let read_count = master.read_buf(&mut buff).await.unwrap_or(0);
        if read_count == 0 {
            panic!("Master closed connection")
        }

        let reply = bytes_to_string(&buff)
            .unwrap_or("".to_string())
            .trim()
            .to_string();

        if reply != "+OK" {
            panic!("Master failed replication");
        }

        Ok(())
    }

    pub async fn send_psync(&mut self, master: &mut TcpStream) -> io::Result<()> {
        let cmd = RESPDatatypes::Array(vec![
            RESPDatatypes::BulkString("PSYNC".to_string()),
            RESPDatatypes::BulkString("?".to_string()),
            RESPDatatypes::BulkString("-1".to_string()),
        ])
        .encode();

        let write_size = master.write(&cmd).await.unwrap_or(0);
        if write_size == 0 {
            panic!("unable to PING master");
        }

        let mut buff = Vec::new();
        let read_count = master.read_buf(&mut buff).await.unwrap_or(0);
        if read_count == 0 {
            panic!("Master closed connection")
        }

        let reply = bytes_to_string(&buff)
            .unwrap_or("".to_string())
            .trim()
            .to_string();

        if reply.starts_with("+FULLRESYNC") {
            let vars: Vec<&str> = reply.split(" ").collect();
            let repl_id = vars.get(1).unwrap().to_string();
            let offset = vars.get(2).unwrap().to_string();

            self.config.replication_config.master_repl_id = Some(repl_id);
            self.config.replication_config.master_repl_offset = Some(offset.parse().unwrap_or(0));
        } else {
            panic!("Invalid response from master");
        }

        Ok(())
    }

    pub async fn event_loop(
        &mut self,
        cache_repo: Arc<Mutex<CacheRepository>>,
        cmd_queue: Arc<Mutex<CmdQueue>>,
    ) {
        // println!("event loop in thread {:?}", std::thread::current().id());
        if let Some(master) = self.initalize().await.unwrap() {
            let stream = (
                master,
                SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            );
            let repo = cache_repo.clone();
            let cmdq = cmd_queue.clone();
            self.event_processor(Ok(stream), repo, cmdq, true);
        }

        loop {
            let repo_clone = cache_repo.clone();

            tokio::spawn(async move {
                clean_cache(repo_clone).await;
            });

            let stream = self.listener.accept().await;
            let repo = cache_repo.clone();
            let cmdq = cmd_queue.clone();

            self.event_processor(stream, repo, cmdq, false);
        }
    }

    fn event_processor(
        &mut self,
        stream: Result<(TcpStream, std::net::SocketAddr), io::Error>,
        repo: Arc<Mutex<CacheRepository>>,
        cmdq: Arc<Mutex<CmdQueue>>,
        is_master: bool,
    ) {
        match stream {
            Ok((stream, addr)) => {
                let mut stm = stream.into_std().unwrap();
                let stream = TcpStream::from_std(stm.try_clone().unwrap()).unwrap();

                // stream cloning
                let mut connection = Connection::new(
                    stream,
                    addr,
                    repo.clone(),
                    self.config.clone(),
                    cmdq.clone(),
                    is_master,
                );
                let jh = tokio::spawn(async move {
                    // println!("connected! and in thread {:?}", std::thread::current().id());
                    connection.process().await;
                });

                tokio::spawn(async move {
                    keep_alive_probe(&mut stm, jh).await;
                });
            }
            Err(err) => {
                println!("error: {}", err);
            }
        }
    }
}

async fn clean_cache(cache_repo: Arc<Mutex<CacheRepository>>) {
    let mut interval = time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        let cache_repo = Arc::clone(&cache_repo);
        cache_repo.lock().await.actively_remove_expired_keys().await;
    }
}

async fn keep_alive_probe(stream: &mut std::net::TcpStream, jh: JoinHandle<()>) {
    let mut interval = time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        if jh.is_finished() {
            // println!("stream is finished");
            break;
        }
        match stream.try_clone().unwrap().write_all(b"") {
            Ok(_) => {}
            Err(_err) => {
                // println!("err {}", err);
                if !jh.is_finished() {
                    jh.abort_handle().abort();
                    while !jh.is_finished() {}
                }
                // println!("will break the loop now");
                break;
            }
        }
    }
}
