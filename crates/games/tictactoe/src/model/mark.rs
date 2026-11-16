/// X always moves first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    X,
    O,
}

impl Mark {
    pub const fn opponent(self) -> Mark {
        match self {
            Mark::X => Mark::O,
            Mark::O => Mark::X,
        }
    }

    pub(crate) const fn code(self) -> u8 {
        match self {
            Mark::X => 1,
            Mark::O => 2,
        }
    }
}
