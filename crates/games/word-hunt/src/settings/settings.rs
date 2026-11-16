use mg_core::{Capped, GameSettings, SettingProblem, SettingRange, SettingsError};

use crate::settings::{Language, Puzzle, PuzzleMode, RawSettings};

pub type WordLength = Capped<4, 7>;
pub type MaxGuesses = Capped<4, 8>;
/// Days since 1970-01-01; the cap is only there to bound the input (year 4707).
pub type DayNumber = Capped<0, 1_000_000>;

/// The word of the day always has this many letters, so every player solves the same puzzle.
pub const DAILY_WORD_LENGTH: u32 = 5;

/// Valid by construction (see [`GameSettings::parse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub language: Language,
    word_length: WordLength,
    pub max_guesses: MaxGuesses,
    pub hard_mode: bool,
    puzzle: Puzzle,
}

impl Settings {
    pub fn word_length(&self) -> WordLength {
        self.word_length
    }

    pub fn puzzle(&self) -> Puzzle {
        self.puzzle
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::English,
            word_length: WordLength::constant::<5>(),
            max_guesses: MaxGuesses::constant::<6>(),
            hard_mode: false,
            puzzle: Puzzle::Unlimited,
        }
    }
}

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        let word_length =
            WordLength::new(raw.word_length).map_err(SettingsError::at("word_length"))?;
        let max_guesses =
            MaxGuesses::new(raw.max_guesses).map_err(SettingsError::at("max_guesses"))?;
        let puzzle = match raw.mode {
            PuzzleMode::Unlimited => Puzzle::Unlimited,
            PuzzleMode::Daily => {
                if word_length.get() != DAILY_WORD_LENGTH {
                    return Err(SettingsError::at("word_length")(
                        SettingProblem::RuleBroken {
                            rule: "the word of the day has a fixed length",
                            value: word_length.get(),
                        },
                    ));
                }
                Puzzle::Daily(DayNumber::new(raw.day).map_err(SettingsError::at("day"))?)
            }
        };
        Ok(Self {
            language: raw.language,
            word_length,
            max_guesses,
            hard_mode: raw.hard_mode,
            puzzle,
        })
    }

    fn to_raw(&self) -> RawSettings {
        let (mode, day) = match self.puzzle {
            Puzzle::Unlimited => (PuzzleMode::Unlimited, 0),
            Puzzle::Daily(day) => (PuzzleMode::Daily, day.get()),
        };
        RawSettings {
            language: self.language,
            word_length: self.word_length.get(),
            max_guesses: self.max_guesses.get(),
            hard_mode: self.hard_mode,
            mode,
            day,
        }
    }

    fn ranges(raw: &RawSettings) -> Vec<SettingRange> {
        match raw.mode {
            PuzzleMode::Unlimited => vec![
                WordLength::range("word_length"),
                MaxGuesses::range("max_guesses"),
            ],
            // The UI reads the daily length from here instead of keeping its own copy.
            PuzzleMode::Daily => vec![
                SettingRange {
                    field: "word_length",
                    min: DAILY_WORD_LENGTH,
                    max: DAILY_WORD_LENGTH,
                },
                MaxGuesses::range("max_guesses"),
                DayNumber::range("day"),
            ],
        }
    }
}
