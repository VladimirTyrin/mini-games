use crate::column::Column;

/// Field edge a crane enters from or leaves towards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// The off-field column just outside this edge.
    pub(crate) const fn outside_column(self, field_width: i32) -> Column {
        match self {
            Side::Left => Column::new(-1),
            Side::Right => Column::new(field_width),
        }
    }
}
