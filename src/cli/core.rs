use clap::Parser;
use rand::{distributions::Alphanumeric, Rng};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about=None)]
pub struct BaseCliArgs {
    #[arg(short, long)]
    port: Option<u16>,
    #[arg(short, long)]
    replicaof: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Roles {
    Master(String, u8),
    Slave(String),
}

impl BaseCliArgs {
    pub fn get_port_or_default(&self, default_port: u16) -> u16 {
        if let Some(port) = self.port {
            return port;
        }

        default_port
    }

    pub fn get_role(&self) -> Roles {
        if let Some(addr) = self.replicaof.as_ref() {
            return Roles::Slave(addr.to_string());
        }
        Roles::Master(self.generate_master_id(), 0)
    }

    pub fn generate_master_id(&self) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect()
    }
}
