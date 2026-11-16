#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Waiting for the authority to choose the answer.
    Choosing,
    Guessing,
    Solved,
    /// Out of guesses: a loss alone, a draw in a group.
    Failed,
    /// Every person left before the end.
    Abandoned,
    /// A group game where everyone else left; the one still in play wins.
    Forfeited,
}

impl Phase {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            Phase::Choosing => 0,
            Phase::Guessing => 1,
            Phase::Solved => 2,
            Phase::Failed => 3,
            Phase::Abandoned => 4,
            Phase::Forfeited => 5,
        }
    }

    pub fn is_over(self) -> bool {
        matches!(
            self,
            Phase::Solved | Phase::Failed | Phase::Abandoned | Phase::Forfeited
        )
    }
}
