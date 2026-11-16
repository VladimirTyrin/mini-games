use mg_core::Slot;

use crate::game_end::GameEnd;
use crate::mark::Mark;
use crate::point::Point;

/// Everything a client renders. The same for every viewer: TicTacToe has no hidden information.
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
}
