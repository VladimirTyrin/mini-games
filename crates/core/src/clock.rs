#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// One frame per accepted move; nothing happens between moves.
    TurnBased,
    /// One frame every `tick_ms`; [`crate::Game::step`] runs once per frame.
    Realtime { tick_ms: u32 },
}

impl Clock {
    pub fn is_realtime(self) -> bool {
        matches!(self, Clock::Realtime { .. })
    }
}
