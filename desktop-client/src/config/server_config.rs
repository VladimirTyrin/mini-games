use common::config::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub address: Option<String>,
    pub disconnect_timeout_ms: u32,
}

impl Validate for ServerConfig {
    fn validate(&self) -> Result<(), String> {
        if let Some(address) = &self.address
            && address.is_empty()
        {
            return Err("server address must not be empty if provided".to_string());
        }
        if self.disconnect_timeout_ms == 0 {
            return Err("disconnect_timeout_ms must be greater than 0".to_string());
        }
        Ok(())
    }
}
