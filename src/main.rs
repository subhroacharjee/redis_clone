use std::sync::Arc;

use cache::core::CacheRepository;
use cmd_queue::core::CmdQueue;
use connections::server::Server;
use tokio::sync::Mutex;

pub mod cache;
pub mod cli;
pub mod cmd_queue;
pub mod command;
pub mod connections;
pub mod errors;
pub mod resp;

#[tokio::main]
async fn main() {
    let mut listener = Server::new().await;
    let cache_repo = Arc::new(Mutex::new(CacheRepository::default()));
    let cmd_queue = Arc::new(Mutex::new(CmdQueue::default()));

    listener.event_loop(cache_repo, cmd_queue).await;
}
