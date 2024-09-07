use std::{
    future::Future,
    io::{self, Error, Result},
    pin::Pin,
    sync::Arc,
};

use tokio::sync::Mutex;

pub type RunResult<'a> = Pin<Box<dyn Future<Output = Result<RESPDatatypes>> + Send + 'a>>;

use crate::{
    cache::core::CacheRepository,
    command::{
        discard::Discard, echo::Echo, exec::Exec, get::Get, incr::Incr, info::Info, multi::Multi,
        ping::Ping, psync::Psync, replconf::ReplConf, set::Set,
    },
    connections::connection::Connection,
    errors::command_not_found::CommandNotFoundError,
    resp::{core::RESPDatatypes, deserialize::Deseralize},
};

pub trait Command: std::marker::Sync + std::marker::Send {
    fn can_execute(&mut self, cmd: &RESPDatatypes) -> bool;
    fn run(
        &mut self,
        cache_repo: Arc<Mutex<CacheRepository>>,
        conn: Option<&mut Connection>,
    ) -> RunResult;
}

fn get_registered_commands() -> Vec<Box<dyn Command>> {
    vec![
        Box::new(Ping::default()),
        Box::new(Echo::default()),
        Box::new(Set::default()),
        Box::new(Get::default()),
        Box::new(Incr::default()),
        Box::new(Multi),
        Box::new(Exec),
        Box::new(Discard),
        Box::new(Info::default()),
        Box::new(ReplConf::default()),
        Box::new(Psync),
    ]
}

pub async fn run(
    input: &mut Vec<u8>,
    cache_repo: Arc<Mutex<CacheRepository>>,
    conn: &mut Connection,
) -> Vec<u8> {
    let dslz = Deseralize {};
    // println!("working");
    match dslz.deseralize(input) {
        Ok(cmd) => run_command(cmd, cache_repo, conn).await,
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Vec::new(),
        Err(err) => RESPDatatypes::SimpleError(Box::new(err)).encode(),
    }
}

pub async fn run_command(
    cmd: RESPDatatypes,
    cache_repo: Arc<Mutex<CacheRepository>>,
    conn: &mut Connection,
) -> Vec<u8> {
    let mut commands = get_registered_commands();
    // println!("{:?}", cmd);
    for command in commands.iter_mut() {
        if command.can_execute(&cmd) {
            return match command.run(cache_repo, Some(conn)).await {
                Ok(data) => {
                    return data.encode();
                }
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    return Vec::new();
                }
                Err(err) => RESPDatatypes::SimpleError(Box::new(err)).encode(),
            };
        }
    }

    RESPDatatypes::SimpleError(Box::new(Error::new(
        io::ErrorKind::InvalidInput,
        CommandNotFoundError {
            cmd: "command not found".to_string(),
        },
    )))
    .encode()
}
