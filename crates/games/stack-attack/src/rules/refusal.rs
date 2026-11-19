use std::fmt;

use mg_core::EnglishText;

/// Why the game refuses a command or world event. A worker takes any command (the seat and queue
/// checks are common ones), so every reason here is input no honest client or authority sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoCraneSpawnDue,
    CraneTargetOutside,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::NoCraneSpawnDue => "no crane spawn is due",
            Self::CraneTargetOutside => "crane target outside the field",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
