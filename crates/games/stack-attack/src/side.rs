/// Field edge a crane enters from or leaves towards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// The off-field column just outside this edge.
    pub(crate) const fn outside_column(self, field_width: i32) -> i32 {
        match self {
            Side::Left => -1,
            Side::Right => field_width,
        }
    }
}
