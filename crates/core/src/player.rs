#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerKind {
    Human,
    /// Built-in bot executed by the match authority.
    Bot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSpec {
    pub kind: PlayerKind,
}

impl PlayerSpec {
    pub const HUMAN: Self = Self {
        kind: PlayerKind::Human,
    };
    pub const BOT: Self = Self {
        kind: PlayerKind::Bot,
    };

    pub fn is_human(self) -> bool {
        self.kind == PlayerKind::Human
    }
}
