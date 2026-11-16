use mg_core::GameSettings;

use crate::settings::{Language, PuzzleMode, Settings};

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSettings {
    pub language: Language,
    pub word_length: u32,
    pub max_guesses: u32,
    /// Every guess must reuse the letters already found: green ones in place, yellow ones
    /// somewhere.
    pub hard_mode: bool,
    pub mode: PuzzleMode,
    /// Only read in [`PuzzleMode::Daily`].
    pub day: u32,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
