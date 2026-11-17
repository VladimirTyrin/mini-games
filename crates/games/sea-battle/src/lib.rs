//! Sea Battle: two seats place a classic fleet unseen, then take turns firing at each other's
//! sea; whoever sinks the whole other fleet wins. The fleets are the authority's secret.

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

mod bots;
mod game;
mod io;
mod model;
mod rules;
mod settings;
mod view;

pub use bots::{BOT_BUDGET, BotKind, HunterBot};
pub use game::SeaBattle;
pub use io::{Command, SecretCommand, WorldEvent};
pub use model::{
    Cell, CellMark, FLEET_LENGTHS, Fleet, FleetError, HiddenFleets, LastShot, Orientation, Phase,
    Ship, ShotOutcome, Waters,
};
pub use settings::{RawSettings, Settings};
pub use view::View;

#[cfg(test)]
mod tests;
