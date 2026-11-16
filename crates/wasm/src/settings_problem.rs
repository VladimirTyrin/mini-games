use mg_core::SetupError;
use mg_registry::CreateError;
use wasm_bindgen::prelude::*;

/// Why settings were rejected. `field` is the wire field name when one setting is at fault, so a
/// form can show `message` next to that control.
#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsProblem {
    pub field: Option<String>,
    pub message: String,
}

impl SettingsProblem {
    pub(crate) fn general(message: String) -> Self {
        Self {
            field: None,
            message,
        }
    }
}

impl From<CreateError> for SettingsProblem {
    fn from(error: CreateError) -> Self {
        match error {
            CreateError::Setup(SetupError::InvalidSetting { field, reason }) => Self {
                field: Some(field.to_owned()),
                message: reason,
            },
            other => Self::general(other.to_string()),
        }
    }
}
