/// Who plays white, which moves first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum WhitePlayer {
    /// Drawn by lot: the authority announces it as [`crate::WorldEvent::WhiteChosen`] in the
    /// first frame; until then nobody may move.
    #[default]
    Random,
    /// The lobby host (slot 0).
    Host,
    /// The other seat (slot 1).
    Guest,
}

impl WhitePlayer {
    pub(crate) fn code(self) -> u8 {
        match self {
            WhitePlayer::Random => 0,
            WhitePlayer::Host => 1,
            WhitePlayer::Guest => 2,
        }
    }
}
