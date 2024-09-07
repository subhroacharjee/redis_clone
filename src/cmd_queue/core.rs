use std::{collections::VecDeque, sync::Arc, time::Duration};

use tokio::sync::Mutex;

use super::node::Node;

#[derive(Debug, Default)]
pub struct CmdQueue {
    pub queue: Arc<Mutex<VecDeque<Node>>>,
}

impl CmdQueue {
    pub async fn add(&mut self, cmd: Vec<u8>) {
        let mut queue = self.queue.lock().await;
        queue.push_back(Node::new(cmd));
    }

    pub async fn get_all_cmds_after_id(
        &self,
        id: Option<String>,
    ) -> Option<(String, Vec<Vec<u8>>)> {
        let mut result = vec![];
        let mut last_id = String::new();
        {
            let queue = self.queue.lock().await;
            if queue.is_empty() {
                return None;
            }
            if let Some(cmd_id) = id.as_ref() {
                if let Some(end) = queue.back() {
                    if end.id == *cmd_id {
                        return Some((cmd_id.to_string(), result));
                    }
                }
                let mut flag = true;

                for element in queue.iter() {
                    if flag && element.id == *cmd_id {
                        flag = false;
                        continue;
                    } else if flag {
                        continue;
                    }
                    result.push(element.cmd.to_vec());
                    last_id = element.id.to_string();
                }
            } else {
                for element in queue.iter() {
                    result.push(element.cmd.to_vec());
                    last_id = element.id.to_string();
                }
            }
        }
        Some((last_id, result))
    }

    pub async fn remove_all_expired_node(&mut self) {
        loop {
            tokio::time::sleep(Duration::from_millis(2)).await;
            {
                let mut queue = self.queue.lock().await;
                while let Some(front) = queue.front() {
                    if !front.has_expired() {
                        break;
                    }

                    queue.pop_front();
                }
            }
        }
    }
}
