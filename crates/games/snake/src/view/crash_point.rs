/// A point on the board in half cells, so that both a cell's centre and the edge between two cells
/// are whole numbers: cell `x` spans `2x..=2x+2` with its centre at `2x+1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrashPoint {
    pub x2: u32,
    pub y2: u32,
}
