use crate::rules::Refusal;

/// Why a guess breaks hard mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardModeViolation {
    /// A letter found in place (green) is missing from its position.
    MovedCorrectLetter,
    /// A letter found elsewhere (yellow), or a found letter's count, is missing.
    DroppedFoundLetter,
}

impl From<HardModeViolation> for Refusal {
    fn from(violation: HardModeViolation) -> Self {
        match violation {
            HardModeViolation::MovedCorrectLetter => Refusal::MovedCorrectLetter,
            HardModeViolation::DroppedFoundLetter => Refusal::DroppedFoundLetter,
        }
    }
}
