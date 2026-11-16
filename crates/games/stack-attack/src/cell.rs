use crate::column::Column;

/// A field cell. `y = 0` is the floor row and `y` grows upwards (towards the cranes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn column(self) -> Column {
        Column::new(self.x)
    }

    pub const fn above(self) -> Self {
        Self::new(self.x, self.y + 1)
    }

    pub const fn below(self) -> Self {
        Self::new(self.x, self.y - 1)
    }

    pub const fn shifted_x(self, dx: i32) -> Self {
        Self::new(self.x + dx, self.y)
    }
}
