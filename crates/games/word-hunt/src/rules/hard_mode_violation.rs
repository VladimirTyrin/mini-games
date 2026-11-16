/// Why a guess breaks hard mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardModeViolation {
    /// A letter found in place (green) is missing from its position.
    MovedCorrectLetter,
    /// A letter found elsewhere (yellow), or a found letter's count, is missing.
    DroppedFoundLetter,
}

impl HardModeViolation {
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            HardModeViolation::MovedCorrectLetter => {
                "hard mode: keep every green letter in its place"
            }
            HardModeViolation::DroppedFoundLetter => "hard mode: use every letter you have found",
        }
    }
}
