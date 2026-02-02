use common::config::Validate;
use common::proto::numbers_match::HintMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct NumbersMatchLobbyConfig {
    pub hint_mode: HintMode,
}

impl Validate for NumbersMatchLobbyConfig {
    fn validate(&self) -> Result<(), String> {
        if self.hint_mode == HintMode::Unspecified {
            return Err("Hint mode must be specified".to_string());
        }
        Ok(())
    }
}

impl Default for NumbersMatchLobbyConfig {
    fn default() -> Self {
        Self {
            hint_mode: HintMode::Limited,
        }
    }
}
