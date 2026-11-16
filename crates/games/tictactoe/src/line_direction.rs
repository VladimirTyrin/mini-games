use crate::point::Point;

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

    /// Column and row delta of one forward step.
    pub(crate) const fn delta(self) -> (i32, i32) {
        match self {
            LineDirection::Horizontal => (1, 0),
            LineDirection::Vertical => (0, 1),
            LineDirection::Diagonal => (1, 1),
            LineDirection::AntiDiagonal => (-1, 1),
        }
    }

    /// `None` when the result would leave the non-negative quadrant.
    pub(crate) fn walk(self, from: Point, steps: i32) -> Option<Point> {
        let (dx, dy) = self.delta();
        let x = i64::from(from.x) + i64::from(dx) * i64::from(steps);
        let y = i64::from(from.y) + i64::from(dy) * i64::from(steps);
        Some(Point::new(u32::try_from(x).ok()?, u32::try_from(y).ok()?))
    }
}
