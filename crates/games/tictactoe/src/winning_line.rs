use crate::mark::Mark;
use crate::point::Point;

/// The whole run of equal marks through the winning move, endpoints inclusive. It can be longer
/// than `win_count`. `start` is the end with the smaller `y` (the left end on a row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WinningLine {
    pub mark: Mark,
    pub start: Point,
    pub end: Point,
}
