//! Type-erased access to every game. Games are listed once, in `games.rs`; every dispatch below
//! is generated from that list.

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
mod dyn_bot_decision;
mod dyn_bot_turn;
mod dyn_mirror;
mod game_kind;
mod games;
mod loaded_replay;
mod match_header;
mod mirror_adapter;
mod rejection;
mod replay_error;
mod replay_file;
mod seat_spec;
mod seats_error;
mod submit_error;
mod typed_bot_turn;

pub use apply_error::ApplyError;
pub use create_error::CreateError;
pub use dyn_authority::DynAuthority;
pub use dyn_bot_decision::DynBotDecision;
pub use dyn_bot_turn::DynBotTurn;
pub use dyn_mirror::DynMirror;
pub use game_kind::GameKind;
pub use loaded_replay::LoadedReplay;
pub use match_header::{players_from_header, seats_from_header};
pub use rejection::Rejection;
pub use replay_error::ReplayError;
pub use replay_file::load_replay_file;
pub use seat_spec::SeatSpec;
pub use seats_error::SeatsError;
pub use submit_error::SubmitError;

use mg_core::{Game, GameSettings, PlayerSpec};
use mg_protocol::{WireError, WireGame, proto};

use games::games;

macro_rules! authority_for {
    (bots, $game:path, $kind:expr, $settings:expr, $seats:expr, $seed:expr) => {
        authority_adapter::with_bots::<$game>($kind, $settings, $seats, $seed)
    };
    (no_bots, $game:path, $kind:expr, $settings:expr, $seats:expr, $seed:expr) => {
        authority_adapter::without_bots::<$game>($kind, $settings, $seats, $seed)
    };
}

macro_rules! bot_kinds_for {
    (bots, $game:path) => {{
        use mg_core::HasBots;
        use mg_protocol::WireBots;
        <$game>::BOT_KINDS
            .iter()
            .map(|kind| <$game>::bot_kind_to_wire(*kind))
            .collect()
    }};
    (no_bots, $game:path) => {
        Vec::new()
    };
}

macro_rules! validate_bot_kind_for {
    (bots, $game:path, $bot:expr, $kind:expr) => {{
        use mg_protocol::WireBots;
        <$game>::bot_kind_from_wire($bot)
            .map(|_| ())
            .map_err(CreateError::from)
    }};
    (no_bots, $game:path, $bot:expr, $kind:expr) => {
        Err(CreateError::NoBots($kind.info().name))
    };
}

macro_rules! define_dispatch {
    ($($kind:ident => $game:path, $settings_case:ident, $bots:ident;)*) => {
        /// The seed must stay with the authority host; clients never see it.
        pub fn create_authority(
            settings: &proto::GameSettings,
            seats: &[SeatSpec],
            seed: u64,
        ) -> Result<Box<dyn DynAuthority>, CreateError> {
            let kind = GameKind::of_settings(settings)?;
            match kind {
                $(GameKind::$kind => authority_for!($bots, $game, kind, settings, seats, seed),)*
            }
        }

        pub fn create_mirror(
            settings: &proto::GameSettings,
            players: &[PlayerSpec],
        ) -> Result<Box<dyn DynMirror>, CreateError> {
            let kind = GameKind::of_settings(settings)?;
            match kind {
                $(GameKind::$kind => mirror_adapter::mirror::<$game>(kind, settings, players),)*
            }
        }

        /// Checks that a bot kind belongs to `kind` and names an existing bot.
        pub fn validate_bot_kind(kind: GameKind, bot: &proto::BotKind) -> Result<(), CreateError> {
            match kind {
                $(GameKind::$kind => validate_bot_kind_for!($bots, $game, bot, kind),)*
            }
        }

        /// Built-in bot kinds offered for a game, in UI order.
        pub fn bot_kinds(kind: GameKind) -> Vec<proto::BotKind> {
            match kind {
                $(GameKind::$kind => bot_kinds_for!($bots, $game),)*
            }
        }

        /// Bounds of the numeric settings as the game enforces them for `settings`, which may be
        /// invalid (a form being edited); only their shape must be right.
        pub fn setting_ranges(
            settings: &proto::GameSettings,
        ) -> Result<Vec<mg_core::SettingRange>, WireError> {
            match GameKind::of_settings(settings)? {
                $(GameKind::$kind => Ok(<<$game as Game>::Settings as GameSettings>::ranges(
                    &<$game>::settings_from_wire(settings)?,
                )),)*
            }
        }

        pub fn default_settings(kind: GameKind) -> proto::GameSettings {
            match kind {
                $(GameKind::$kind => <$game>::settings_to_wire(
                    &<<$game as Game>::Settings>::default().to_raw(),
                ),)*
            }
        }
    };
}

games!(define_dispatch);

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

/// Wire settings to the game's parsed settings: shape errors as [`CreateError::Wire`], rule
/// errors (ranges included) as [`CreateError::Settings`].
pub fn parse_settings<G: WireGame>(
    settings: &proto::GameSettings,
) -> Result<G::Settings, CreateError> {
    Ok(G::Settings::parse(G::settings_from_wire(settings)?)?)
}

fn settings_of(header: &proto::MatchHeader) -> Result<&proto::GameSettings, WireError> {
    header
        .settings
        .as_ref()
        .ok_or(WireError::Missing("header.settings"))
}
