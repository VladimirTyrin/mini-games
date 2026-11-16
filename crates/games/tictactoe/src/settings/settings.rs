use mg_core::{PlayerSpec, SetupError};

use crate::settings::{Dimensions, FirstPlayerMode, Rules, WinLength};

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
        self.rules(players).map(drop)
    }

    pub(crate) fn rules(&self, players: &[PlayerSpec]) -> Result<Rules, SetupError> {
        SetupError::check_player_count(players.len(), PLAYERS, PLAYERS)?;
        let dimensions = Dimensions::new(self.field_width, self.field_height)?;
        let win_length = WinLength::new(self.win_count, dimensions)?;
        Ok(Rules {
            dimensions,
            win_length,
        })
    }
}
