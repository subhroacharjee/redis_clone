use std::time::{Duration, Instant};

use ulid::Ulid;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub cmd: Vec<u8>,
    pub ins: Instant,
}

impl Node {
    pub fn new(cmd: Vec<u8>) -> Node {
        let id = Ulid::new().to_string();
        let exp = Instant::now();
        Node { id, cmd, ins: exp }
    }

    pub fn has_expired(&self) -> bool {
        self.ins + Duration::from_millis(5) < Instant::now()
    }
}
