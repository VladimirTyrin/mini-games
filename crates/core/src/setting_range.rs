use crate::setup_error::SetupError;

/// Inclusive bounds of one numeric setting. Games validate with [`SettingRange::check`] and
/// publish the same values through [`crate::Game::setting_ranges`], so a UI never restates them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingRange {
    /// Wire field name, the same one [`SetupError::InvalidSetting`] reports.
    pub field: &'static str,
    pub min: u32,
    pub max: u32,
}

impl SettingRange {
    pub const fn new(field: &'static str, min: u32, max: u32) -> Self {
        Self { field, min, max }
    }

    pub fn check(self, value: u32) -> Result<(), SetupError> {
        SetupError::check_range(self.field, value, self.min, self.max)
    }
}
