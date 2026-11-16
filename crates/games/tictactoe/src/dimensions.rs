use mg_core::SetupError;

use crate::line_direction::LineDirection;
use crate::line_span::LineSpan;
use crate::point::Point;
use crate::settings::{MAX_FIELD_SIZE, MIN_FIELD_SIZE};

/// Validated board size: both sides in `MIN_FIELD_SIZE..=MAX_FIELD_SIZE`, so they fit a `u8`
/// and every coordinate and cell index converts losslessly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dimensions {
    width: u8,
    height: u8,
}

impl Dimensions {
    pub(crate) fn new(field_width: u32, field_height: u32) -> Result<Self, SetupError> {
        Ok(Self {
            width: side("field_width", field_width)?,
            height: side("field_height", field_height)?,
        })
    }

    pub(crate) fn width(self) -> u32 {
        u32::from(self.width)
    }

    pub(crate) fn height(self) -> u32 {
        u32::from(self.height)
    }

    pub(crate) fn shorter_side(self) -> u32 {
        u32::from(self.width.min(self.height))
    }

    pub(crate) fn cell_count(self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }

    pub(crate) fn centre(self) -> Point {
        Point::new(self.width() / 2, self.height() / 2)
    }

    /// Row-major index, `None` off the board.
    pub(crate) fn index_of(self, point: Point) -> Option<usize> {
        let x = u8::try_from(point.x).ok().filter(|&x| x < self.width)?;
        let y = u8::try_from(point.y).ok().filter(|&y| y < self.height)?;
        Some(usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    /// `None` when `point` is off the board.
    pub(crate) fn line_span(self, point: Point, direction: LineDirection) -> Option<LineSpan> {
        let x = u8::try_from(point.x).ok().filter(|&x| x < self.width)?;
        let y = u8::try_from(point.y).ok().filter(|&y| y < self.height)?;
        let left = usize::from(x);
        let right = usize::from(self.width - 1 - x);
        let up = usize::from(y);
        let down = usize::from(self.height - 1 - y);
        let width = usize::from(self.width);
        let span = |backward_steps, forward_steps, stride| LineSpan {
            origin_index: up * width + left,
            backward_steps,
            forward_steps,
            stride,
        };
        Some(match direction {
            LineDirection::Horizontal => span(left, right, 1),
            LineDirection::Vertical => span(up, down, width),
            LineDirection::Diagonal => span(left.min(up), right.min(down), width + 1),
            LineDirection::AntiDiagonal => span(right.min(up), left.min(down), width - 1),
        })
    }

    /// Every cell in row-major order, so the n-th point has index n.
    pub(crate) fn points(self) -> impl Iterator<Item = Point> {
        (0..self.height()).flat_map(move |y| (0..self.width()).map(move |x| Point::new(x, y)))
    }
}

fn side(field: &'static str, value: u32) -> Result<u8, SetupError> {
    SetupError::check_range(field, value, MIN_FIELD_SIZE, MAX_FIELD_SIZE)?;
    u8::try_from(value).map_err(|error| SetupError::setting(field, error.to_string()))
}
