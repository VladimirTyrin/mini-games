/// One of the four diagonals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Direction {
    dx: i8,
    dy: i8,
}

impl Direction {
    pub(crate) const ALL: [Direction; 4] = [
        Direction { dx: 1, dy: 1 },
        Direction { dx: -1, dy: 1 },
        Direction { dx: 1, dy: -1 },
        Direction { dx: -1, dy: -1 },
    ];

    pub(crate) fn dx(self) -> i8 {
        self.dx
    }

    pub(crate) fn dy(self) -> i8 {
        self.dy
    }
}
