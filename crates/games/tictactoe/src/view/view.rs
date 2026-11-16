use mg_core::Slot;

use crate::model::{Mark, Point};
use crate::rules::GameEnd;

/// Everything a client renders. TicTacToe has no hidden information: views differ only in `you`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub field_width: u32,
    pub field_height: u32,
    pub win_count: u32,
    /// Row-major, `field_width * field_height` entries.
    pub cells: Vec<Option<Mark>>,
    /// `None` until the first player is chosen ([`crate::FirstPlayerMode::Random`]).
    pub x_player: Option<Slot>,
    pub o_player: Option<Slot>,
    /// `None` while the first player is not chosen yet and after the game ended.
    pub current_player: Option<Slot>,
    pub last_move: Option<Point>,
    pub end: Option<GameEnd>,
    /// The viewer's own seat; `None` for spectators.
    pub you: Option<Slot>,
}
