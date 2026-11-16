use std::collections::VecDeque;

use crate::field::Field;
use crate::point::Point;

pub const START_LENGTH: u16 = 3;

/// Heads share the middle row and face `Up` with the body trailing downwards; seat `i` of `n`
/// gets column `(2i + 1) * width / (2n)`. Neighbouring columns differ by `width / n >= 1`
/// (width >= 10 >= seats), so starting bodies never overlap — legacy put two snakes on the last
/// column with 10 seats on width 10. `None` only if the field cannot hold the body.
pub(crate) fn start_body(seat: usize, seats: usize, field: Field) -> Option<VecDeque<Point>> {
    let numerator = (2 * seat + 1).checked_mul(usize::from(field.width()))?;
    let column = u16::try_from(numerator.checked_div(2 * seats)?).ok()?;
    let head_row = field.height() / 2;
    let body: VecDeque<Point> = (0..START_LENGTH)
        .map(|offset| Point::new(column, head_row + offset))
        .collect();
    body.iter()
        .all(|&point| field.contains(point))
        .then_some(body)
}
