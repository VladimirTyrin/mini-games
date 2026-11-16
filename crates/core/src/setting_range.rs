/// Inclusive bounds of one numeric setting, as published to a UI by
/// [`crate::GameSettings::ranges`]. Built from the setting's [`crate::Capped`] type
/// ([`crate::Capped::range`]), so the bounds are stated once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingRange {
    /// Wire field name, the same one [`crate::SettingsError::field`] reports.
    pub field: &'static str,
    pub min: u32,
    pub max: u32,
}

impl SettingRange {
    pub const fn new(field: &'static str, min: u32, max: u32) -> Self {
        Self { field, min, max }
    }
}
