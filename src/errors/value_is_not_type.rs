use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct ValueIsNotType {
    pub type_name: String,
    pub can_be_out_of_range: Option<bool>,
}

impl Error for ValueIsNotType {}

impl Display for ValueIsNotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out_of_range_msg = "";
        if let Some(val) = self.can_be_out_of_range {
            if val {
                out_of_range_msg = " or out of range"
            }
        }

        let msg = format!("ERR value is not an {}{}", self.type_name, out_of_range_msg);
        write!(f, "{}", msg)
    }
}
