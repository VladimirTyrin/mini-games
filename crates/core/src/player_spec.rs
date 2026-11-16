use crate::player_kind::PlayerKind;

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
