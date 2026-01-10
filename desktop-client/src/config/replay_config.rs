use common::config::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ReplayConfig {
    pub save: bool,
    pub location: String,
}

impl Validate for ReplayConfig {
    fn validate(&self) -> Result<(), String> {
        if self.location.is_empty() {
            return Err("replay location must not be empty".to_string());
        }
        Ok(())
    }
}
