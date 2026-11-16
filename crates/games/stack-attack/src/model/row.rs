/// A row counted from the floor (row 0) towards the cranes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Row(i32);

impl Row {
    pub const fn new(y: i32) -> Self {
        Self(y)
    }

    pub const fn get(self) -> i32 {
        self.0
    }
}
