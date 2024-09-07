use std::fmt::Display;

#[derive(Debug)]
pub struct CommandNotFoundError {
    pub cmd: String,
}

impl CommandNotFoundError {
    pub fn get_output(&self) -> String {
        format!("invalid command {}", self.cmd)
    }
}

impl std::error::Error for CommandNotFoundError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl Display for CommandNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cmd = self.cmd.to_string();
        write!(f, "invalid command {cmd}")
    }
}
