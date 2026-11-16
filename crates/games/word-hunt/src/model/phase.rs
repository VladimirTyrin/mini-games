#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Waiting for the authority to choose the answer.
    Choosing,
    Guessing,
    Solved,
    /// Out of guesses.
    Failed,
    /// The player left before the end.
    Abandoned,
}

impl Phase {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            Phase::Choosing => 0,
            Phase::Guessing => 1,
            Phase::Solved => 2,
            Phase::Failed => 3,
            Phase::Abandoned => 4,
        }
    }

    pub fn is_over(self) -> bool {
        matches!(self, Phase::Solved | Phase::Failed | Phase::Abandoned)
    }
}
