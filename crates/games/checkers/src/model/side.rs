/// The colour a seat plays. White starts on rows 0–2 and moves up (towards row 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn opponent(self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }

    /// Row step of a man's move forward.
    pub(crate) fn forward(self) -> i8 {
        match self {
            Side::White => 1,
            Side::Black => -1,
        }
    }

    /// The row where this side's men become kings.
    pub(crate) fn crowning_row(self) -> u8 {
        match self {
            Side::White => 7,
            Side::Black => 0,
        }
    }

    pub(crate) fn code(self) -> u8 {
        match self {
            Side::White => 1,
            Side::Black => 2,
        }
    }
}
