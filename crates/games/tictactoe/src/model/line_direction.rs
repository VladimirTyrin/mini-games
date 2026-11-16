use crate::model::Point;

/// The four line orientations, in the order win detection checks them. Stepping forward always
/// increases `y` (or `x` on a row), so walking backward finds a line's `start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineDirection {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

impl LineDirection {
    pub(crate) const ALL: [LineDirection; 4] = [
        LineDirection::Horizontal,
        LineDirection::Vertical,
        LineDirection::Diagonal,
        LineDirection::AntiDiagonal,
    ];

    /// `None` when the result would leave the non-negative quadrant.
    pub(crate) fn walk(self, from: Point, steps: i32) -> Option<Point> {
        let (dx, dy): (i32, i32) = match self {
            LineDirection::Horizontal => (1, 0),
            LineDirection::Vertical => (0, 1),
            LineDirection::Diagonal => (1, 1),
            LineDirection::AntiDiagonal => (-1, 1),
        };
        from.offset(dx.checked_mul(steps)?, dy.checked_mul(steps)?)
    }
}
