use std::fmt;

use crate::english_text::EnglishText;

/// A rule on one setting that a range cannot express. Generic kinds, so a client words each
/// once for every game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingRule {
    /// The value must be a power of two (2048's target tile).
    PowerOfTwo,
    /// The value cannot be reached on a board of the chosen size (2048's target tile).
    TooLargeForBoard,
    /// The daily puzzle fixes this value (Word Hunt's word length).
    FixedForDailyPuzzle,
}

impl EnglishText for SettingRule {
    fn english(self) -> &'static str {
        match self {
            Self::PowerOfTwo => "must be a power of two",
            Self::TooLargeForBoard => "cannot be reached on a board this small",
            Self::FixedForDailyPuzzle => "is fixed for the daily puzzle",
        }
    }
}

impl fmt::Display for SettingRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
