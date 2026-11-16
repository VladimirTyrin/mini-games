use std::fmt;

/// A number outside its inclusive bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfRange {
    pub value: u32,
    pub min: u32,
    pub max: u32,
}

impl OutOfRange {
    /// The one bounds check: [`crate::Capped`] uses it for bounds fixed in the type, and a game
    /// for a bound that depends on another setting (TicTacToe's win length and its board).
    pub const fn check(value: u32, min: u32, max: u32) -> Result<u32, Self> {
        if value < min || value > max {
            Err(Self { value, min, max })
        } else {
            Ok(value)
        }
    }
}

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "must be between {} and {}, got {}",
            self.min, self.max, self.value
        )
    }
}

impl std::error::Error for OutOfRange {}
