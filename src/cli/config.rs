use super::core::Roles;

pub trait ToConfigString {
    fn to_config_string(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub role: Roles,
    pub master_repl_id: Option<String>,
    pub master_repl_offset: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

impl ReplicationConfig {
    fn convert_role_to_string(&self) -> String {
        match &self.role {
            Roles::Master(id, offset) => format!(
                "role:master\nmaster_replid:{}\nmaster_repl_offset:{}",
                id.clone(),
                offset
            ),
            Roles::Slave(_) => "role:slave".to_string(),
        }
    }
}

impl ToConfigString for ReplicationConfig {
    fn to_config_string(&self) -> String {
        format!("# Replication\n{}", self.convert_role_to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub server_config: ServerConfig,
    pub replication_config: ReplicationConfig,
}
