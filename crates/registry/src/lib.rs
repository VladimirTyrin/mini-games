//! Type-erased access to every game. This is the only place that lists all games: adding a game
//! adds one arm to each `match` here and in `GameKind` (`cargo xtask new-game` does it).

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::unreachable,
        clippy::todo
    )
)]

mod apply_error;
mod authority_adapter;
mod create_error;
mod dyn_authority;
mod dyn_mirror;
mod game_kind;
mod loaded_replay;
mod match_header;
mod mirror_adapter;
mod replay_error;
mod replay_file;
mod seat_spec;
mod seats_error;
mod submit_error;

pub use apply_error::ApplyError;
pub use create_error::CreateError;
pub use dyn_authority::DynAuthority;
pub use dyn_mirror::DynMirror;
pub use game_kind::GameKind;
pub use loaded_replay::LoadedReplay;
pub use match_header::{players_from_header, seats_from_header};
pub use replay_error::ReplayError;
pub use replay_file::load_replay_file;
pub use seat_spec::SeatSpec;
pub use seats_error::SeatsError;
pub use submit_error::SubmitError;

use mg_core::PlayerSpec;
use mg_numbers_match::NumbersMatch;
use mg_protocol::{WireError, proto};
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
        GameKind::Snake => authority_adapter::with_bots::<Snake>(kind, settings, seats, seed),
        GameKind::TicTacToe => {
            authority_adapter::with_bots::<TicTacToe>(kind, settings, seats, seed)
        }
        GameKind::StackAttack => {
            authority_adapter::without_bots::<StackAttack>(kind, settings, seats, seed)
        }
        GameKind::Puzzle2048 => {
            authority_adapter::without_bots::<Puzzle2048>(kind, settings, seats, seed)
        }
        GameKind::NumbersMatch => {
            authority_adapter::without_bots::<NumbersMatch>(kind, settings, seats, seed)
        }
    }
}

pub fn create_mirror(
    settings: &proto::GameSettings,
    players: &[PlayerSpec],
) -> Result<Box<dyn DynMirror>, CreateError> {
    let kind = GameKind::of_settings(settings)?;
    match kind {
        GameKind::Snake => mirror_adapter::mirror::<Snake>(kind, settings, players),
        GameKind::TicTacToe => mirror_adapter::mirror::<TicTacToe>(kind, settings, players),
        GameKind::StackAttack => mirror_adapter::mirror::<StackAttack>(kind, settings, players),
        GameKind::Puzzle2048 => mirror_adapter::mirror::<Puzzle2048>(kind, settings, players),
        GameKind::NumbersMatch => mirror_adapter::mirror::<NumbersMatch>(kind, settings, players),
    }
}

/// The seed must stay with the authority host; clients never see it.
pub fn create_authority_from_header(
    header: &proto::MatchHeader,
    seed: u64,
) -> Result<Box<dyn DynAuthority>, CreateError> {
    create_authority(settings_of(header)?, &seats_from_header(header)?, seed)
}

pub fn create_mirror_from_header(
    header: &proto::MatchHeader,
) -> Result<Box<dyn DynMirror>, CreateError> {
    create_mirror(settings_of(header)?, &players_from_header(header)?)
}

fn settings_of(header: &proto::MatchHeader) -> Result<&proto::GameSettings, WireError> {
    header
        .settings
        .as_ref()
        .ok_or(WireError::Missing("header.settings"))
}

/// Checks that a bot kind belongs to `kind` and names an existing bot.
pub fn validate_bot_kind(kind: GameKind, bot: &proto::BotKind) -> Result<(), CreateError> {
    use mg_protocol::WireBots;
    match kind {
        GameKind::Snake => Snake::bot_kind_from_wire(bot).map(|_| ())?,
        GameKind::TicTacToe => TicTacToe::bot_kind_from_wire(bot).map(|_| ())?,
        GameKind::StackAttack | GameKind::Puzzle2048 | GameKind::NumbersMatch => {
            return Err(CreateError::NoBots(kind.info().name));
        }
    }
    Ok(())
}

/// Built-in bot kinds offered for a game, in UI order.
pub fn bot_kinds(kind: GameKind) -> Vec<proto::BotKind> {
    use mg_core::HasBots;
    use mg_protocol::WireBots;
    match kind {
        GameKind::Snake => Snake::BOT_KINDS
            .iter()
            .map(|k| Snake::bot_kind_to_wire(*k))
            .collect(),
        GameKind::TicTacToe => TicTacToe::BOT_KINDS
            .iter()
            .map(|k| TicTacToe::bot_kind_to_wire(*k))
            .collect(),
        GameKind::StackAttack | GameKind::Puzzle2048 | GameKind::NumbersMatch => Vec::new(),
    }
}

pub fn default_settings(kind: GameKind) -> proto::GameSettings {
    use mg_protocol::WireGame;
    match kind {
        GameKind::Snake => Snake::settings_to_wire(&Default::default()),
        GameKind::TicTacToe => TicTacToe::settings_to_wire(&Default::default()),
        GameKind::StackAttack => StackAttack::settings_to_wire(&Default::default()),
        GameKind::Puzzle2048 => Puzzle2048::settings_to_wire(&Default::default()),
        GameKind::NumbersMatch => NumbersMatch::settings_to_wire(&Default::default()),
    }
}
