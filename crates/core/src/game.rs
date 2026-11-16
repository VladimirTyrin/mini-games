//! The [`Game`] trait every game implements, plus the small vocabulary types around it.

use std::fmt::Debug;

use crate::error::{CommandError, SetupError};
use crate::hash::StateHasher;
use crate::rng::CoreRng;

/// Player seat index. Seats are assigned in lobby join order and never change during a match.
pub type Slot = u8;

/// Who occupies a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerKind {
    /// A person (local or remote).
    Human,
    /// A built-in bot run by the match authority.
    Bot,
}

/// Seat description passed to [`Game::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSpec {
    /// Human or bot.
    pub kind: PlayerKind,
}

impl PlayerSpec {
    /// A human seat.
    pub const HUMAN: Self = Self {
        kind: PlayerKind::Human,
    };
    /// A bot seat.
    pub const BOT: Self = Self {
        kind: PlayerKind::Bot,
    };
}

/// How a game advances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// Advances only when commands arrive; one frame per accepted move.
    TurnBased,
    /// Advances every `tick_ms` milliseconds; [`Game::step`] runs once per frame.
    Realtime {
        /// Tick length in milliseconds (> 0).
        tick_ms: u32,
    },
}

/// Whose input the game is waiting for (drives bot scheduling).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Awaiting {
    /// Exactly this seat may act now.
    Turn(Slot),
    /// Every living seat may act (real-time games).
    Simultaneous,
    /// Nobody (the game is over).
    Nobody,
}

/// How a finished game ended, from the table's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind {
    /// Competitive game with winner(s) in [`Outcome::winners`].
    Winner,
    /// Competitive game without a winner.
    Draw,
    /// Solo/cooperative game the players completed.
    Victory,
    /// Solo/cooperative game the players lost (or "game over" with a score).
    Defeat,
}

/// Final result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Classification.
    pub kind: OutcomeKind,
    /// Winning seats (empty unless `kind == Winner`).
    pub winners: Vec<Slot>,
    /// Score per seat, indexed by [`Slot`].
    pub scores: Vec<i64>,
}

/// Match status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Still running.
    InProgress,
    /// Finished with this outcome.
    Finished(Outcome),
}

impl Status {
    /// `true` once finished.
    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Finished(_))
    }
}

/// Whose eyes a [`Game::view`] is rendered for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viewer {
    /// A seated player.
    Player(Slot),
    /// An observer or replay viewer.
    Spectator,
}

/// Static facts about a game, used by lobbies and UIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameInfo {
    /// Stable machine id (`snake`, `tictactoe`, ...). Never reuse or rename.
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Minimum seats (humans + bots).
    pub min_players: u8,
    /// Maximum seats (humans + bots).
    pub max_players: u8,
    /// Whether built-in bots exist for this game.
    pub has_bots: bool,
    /// Players win or lose together (solo games count as cooperative).
    pub cooperative: bool,
}

/// A deterministic game.
///
/// # Determinism contract
/// * `apply_command`, `apply_world`, `step` and `drop_player` are pure functions of the current
///   state and their arguments: no clocks, no global RNG, no `HashMap` iteration, no floats.
/// * All randomness is produced by [`Game::roll_world`] on the authority and delivered to everyone
///   as `WorldEvent`s, so clients never need the seed.
/// * A rejected command or world event must leave the state untouched.
/// * [`Game::hash_state`] feeds every gameplay-relevant field in a fixed order.
pub trait Game: Sized + Send + 'static {
    /// Validated lobby settings.
    type Settings: Clone + Debug + Send + Sync + 'static;
    /// A player's (or bot's) input.
    type Command: Clone + Debug + PartialEq + Send + Sync + 'static;
    /// Randomness resolved by the authority.
    type WorldEvent: Clone + Debug + PartialEq + Send + Sync + 'static;
    /// Render model.
    type View: Debug;

    /// Static facts.
    const INFO: GameInfo;

    /// Builds the initial state. Must reject any invalid settings or seat list without
    /// allocating proportionally to untrusted sizes.
    fn new(settings: &Self::Settings, players: &[PlayerSpec]) -> Result<Self, SetupError>;

    /// Tick model.
    fn clock(&self) -> Clock;

    /// Validates and applies a command atomically.
    fn apply_command(&mut self, slot: Slot, cmd: &Self::Command) -> Result<(), CommandError>;

    /// Authority only: draws whatever randomness the current state needs (may be empty).
    /// Called once per frame, after that frame's commands.
    fn roll_world(&self, rng: &mut CoreRng) -> Vec<Self::WorldEvent>;

    /// Applies a world event produced by [`Game::roll_world`].
    fn apply_world(&mut self, event: &Self::WorldEvent) -> Result<(), CommandError>;

    /// Advances one tick (real-time games only; turn-based games keep the default no-op).
    fn step(&mut self) {}

    /// Removes a seat from play (disconnect budget exhausted, left, kicked).
    fn drop_player(&mut self, slot: Slot);

    /// Whose input is expected.
    fn awaiting(&self) -> Awaiting;

    /// Running or finished.
    fn status(&self) -> Status;

    /// Render model for a viewer.
    fn view(&self, viewer: Viewer) -> Self::View;

    /// Feeds the canonical state into `h`.
    fn hash_state(&self, h: &mut StateHasher);

    /// Convenience: the canonical state hash.
    fn state_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        self.hash_state(&mut h);
        h.finish()
    }
}
