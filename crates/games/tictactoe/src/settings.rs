use mg_core::{PlayerSpec, SetupError};

use crate::first_player_mode::FirstPlayerMode;

pub const MIN_FIELD_SIZE: u32 = 3;
pub const MAX_FIELD_SIZE: u32 = 20;
/// The upper bound is `min(field_width, field_height)`.
pub const MIN_WIN_COUNT: u32 = 3;
pub const PLAYERS: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    pub field_width: u32,
    pub field_height: u32,
    /// Marks in a row needed to win.
    pub win_count: u32,
    pub first_player: FirstPlayerMode,
}

impl Default for Settings {
    /// Web client defaults.
    fn default() -> Self {
        Self {
            field_width: 15,
            field_height: 15,
            win_count: 5,
            first_player: FirstPlayerMode::Random,
        }
    }
}

impl Settings {
    pub fn validate(&self, players: &[PlayerSpec]) -> Result<(), SetupError> {
        SetupError::check_player_count(players.len(), PLAYERS, PLAYERS)?;
        SetupError::check_range(
            "field_width",
            self.field_width,
            MIN_FIELD_SIZE,
            MAX_FIELD_SIZE,
        )?;
        SetupError::check_range(
            "field_height",
            self.field_height,
            MIN_FIELD_SIZE,
            MAX_FIELD_SIZE,
        )?;
        SetupError::check_range(
            "win_count",
            self.win_count,
            MIN_WIN_COUNT,
            self.field_width.min(self.field_height),
        )
    }
}
