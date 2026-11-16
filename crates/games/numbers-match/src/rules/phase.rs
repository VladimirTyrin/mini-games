use crate::rules::LossReason;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Until the authority's first frame deals the board; commands are rejected meanwhile.
    Dealing,
    Playing,
    Won,
    Lost(LossReason),
}

impl Phase {
    pub fn is_over(self) -> bool {
        matches!(self, Phase::Won | Phase::Lost(_))
    }
}
