use super::stone::Stone;

/// Search perspective: the bot maximizes, its opponent minimizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Bot,
    Opponent,
}

impl Side {
    pub(crate) const fn other(self) -> Side {
        match self {
            Side::Bot => Side::Opponent,
            Side::Opponent => Side::Bot,
        }
    }

    pub(crate) const fn stone(self) -> Stone {
        match self {
            Side::Bot => Stone::Bot,
            Side::Opponent => Stone::Opponent,
        }
    }
}
