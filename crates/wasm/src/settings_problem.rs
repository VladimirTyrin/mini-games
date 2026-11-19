use mg_protocol::setting_problem_to_wire;
use mg_registry::CreateError;
use prost::Message;
use wasm_bindgen::prelude::*;

/// Why settings were rejected. `field` is the wire field name when one setting is at fault, so a
/// form can show the problem next to that control; `settingProblem` is then the encoded
/// `SettingProblem` for the page to word. `message` is English, for the console.
#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsProblem {
    pub field: Option<String>,
    #[wasm_bindgen(js_name = settingProblem)]
    pub setting_problem: Option<Vec<u8>>,
    pub message: String,
}

impl SettingsProblem {
    pub(crate) fn general(message: String) -> Self {
        Self {
            field: None,
            setting_problem: None,
            message,
        }
    }
}

impl From<CreateError> for SettingsProblem {
    fn from(error: CreateError) -> Self {
        match error {
            CreateError::Settings(error) => Self {
                field: Some(error.field.to_owned()),
                setting_problem: Some(setting_problem_to_wire(&error).encode_to_vec()),
                message: error.problem.to_string(),
            },
            other => Self::general(other.to_string()),
        }
    }
}
