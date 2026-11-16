use mg_core::SetupError;

pub const MIN_FIELD_WIDTH: u32 = 6;
pub const MAX_FIELD_WIDTH: u32 = 32;
/// Leaves room for a worker (2 cells), a jump and the ceiling row.
pub const MIN_FIELD_HEIGHT: u32 = 6;
pub const MAX_FIELD_HEIGHT: u32 = 24;
pub const MIN_SEATS: u8 = 1;
pub const MAX_SEATS: u8 = 4;

/// The legacy lobby had no settings (fixed 15x10 field); the size is configurable now and the
/// default keeps the legacy field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    pub field_width: u32,
    pub field_height: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            field_width: 15,
            field_height: 10,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), SetupError> {
        SetupError::check_range(
            "field_width",
            self.field_width,
            MIN_FIELD_WIDTH,
            MAX_FIELD_WIDTH,
        )?;
        SetupError::check_range(
            "field_height",
            self.field_height,
            MIN_FIELD_HEIGHT,
            MAX_FIELD_HEIGHT,
        )
    }
}
