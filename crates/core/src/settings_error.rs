use std::fmt;

use crate::setting_problem::SettingProblem;

/// Why [`crate::GameSettings::parse`] rejected raw settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsError {
    /// Wire field name, the same one [`crate::SettingRange::field`] publishes, so a form can show
    /// the problem next to its control.
    pub field: &'static str,
    pub problem: SettingProblem,
}

impl SettingsError {
    /// For `map_err`: `FieldSize::new(raw.width).map_err(SettingsError::at("width"))?`.
    pub fn at<P: Into<SettingProblem>>(field: &'static str) -> impl FnOnce(P) -> Self {
        move |problem| Self {
            field,
            problem: problem.into(),
        }
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid setting `{}`: {}", self.field, self.problem)
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.problem)
    }
}
