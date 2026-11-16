#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Playing,
    /// A tile reached the target (`OutcomeKind::Victory`).
    Won,
    /// No move can change the board (`OutcomeKind::Defeat`; the legacy UI showed it as a draw).
    Lost,
    /// The player left or was dropped (`OutcomeKind::Defeat`).
    Abandoned,
}

impl Phase {
    /// Feeds the state hash: never renumber.
    pub(crate) fn code(self) -> u8 {
        match self {
            Phase::Playing => 0,
            Phase::Won => 1,
            Phase::Lost => 2,
            Phase::Abandoned => 3,
        }
    }
}
