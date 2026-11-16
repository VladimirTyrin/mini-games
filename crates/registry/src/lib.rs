//! Type-erased access to every game. This is the only place that lists all games: adding a game
//! adds one arm to each `match` here and in `GameKind` (`cargo xtask new-game` does it).

mod dyn_authority;
mod dyn_mirror;
mod game_kind;
mod registry_error;
mod seat_spec;

pub use dyn_authority::DynAuthority;
pub use dyn_mirror::DynMirror;
pub use game_kind::GameKind;
pub use registry_error::{ApplyError, CreateError, SubmitError};
pub use seat_spec::SeatSpec;

use mg_core::PlayerSpec;
use mg_numbers_match::NumbersMatch;
use mg_protocol::proto;
use mg_puzzle2048::Puzzle2048;
use mg_snake::Snake;
use mg_stack_attack::StackAttack;
use mg_tictactoe::TicTacToe;

/// The seed must stay with the authority host; clients never see it.
pub fn create_authority(
    settings: &proto::GameSettings,
    seats: &[SeatSpec],
    seed: u64,
) -> Result<Box<dyn DynAuthority>, CreateError> {
    let kind = GameKind::of_settings(settings)?;
    match kind {
        GameKind::Snake => dyn_authority::with_bots::<Snake>(kind, settings, seats, seed),
        GameKind::TicTacToe => dyn_authority::with_bots::<TicTacToe>(kind, settings, seats, seed),
        GameKind::StackAttack => {
            dyn_authority::without_bots::<StackAttack>(kind, settings, seats, seed)
        }
        GameKind::Puzzle2048 => {
            dyn_authority::without_bots::<Puzzle2048>(kind, settings, seats, seed)
        }
        GameKind::NumbersMatch => {
            dyn_authority::without_bots::<NumbersMatch>(kind, settings, seats, seed)
        }
    }
}

pub fn create_mirror(
    settings: &proto::GameSettings,
    players: &[PlayerSpec],
) -> Result<Box<dyn DynMirror>, CreateError> {
    let kind = GameKind::of_settings(settings)?;
    match kind {
        GameKind::Snake => dyn_mirror::mirror::<Snake>(kind, settings, players),
        GameKind::TicTacToe => dyn_mirror::mirror::<TicTacToe>(kind, settings, players),
        GameKind::StackAttack => dyn_mirror::mirror::<StackAttack>(kind, settings, players),
        GameKind::Puzzle2048 => dyn_mirror::mirror::<Puzzle2048>(kind, settings, players),
        GameKind::NumbersMatch => dyn_mirror::mirror::<NumbersMatch>(kind, settings, players),
    }
}
