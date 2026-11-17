use std::fmt::Debug;

use crate::awaiting::Awaiting;
use crate::clock::Clock;
use crate::command_error::CommandError;
use crate::game_info::GameInfo;
use crate::game_secret::{GameSecret, SeatSecret, SecretCommand};
use crate::game_settings::GameSettings;
use crate::hash::StateHasher;
use crate::player_spec::PlayerSpec;
use crate::rng::CoreRng;
use crate::setup_error::SetupError;
use crate::slot::Slot;
use crate::status::Status;
use crate::viewer::Viewer;

/// Rules of one game.
///
/// Determinism contract (lockstep relies on it, the conformance suite checks it):
/// * every state transition is a pure function of the state and its arguments: no clocks, no
///   global RNG, no `HashMap` iteration, no floats;
/// * randomness exists only in [`Game::roll_secret`] and [`Game::roll_world`], which run on the
///   authority; their results travel as `WorldEvent`s, so clients never need the seed;
/// * what only the authority may know (a hidden word, the fleets) lives in [`Game::Secret`],
///   outside the game state: it is never hashed and reaches clients only as the events the game
///   chooses to derive from it;
/// * a rejected command or world event leaves the state untouched;
/// * [`Game::hash_state`] covers every gameplay-relevant field in a fixed order.
///
/// `Clone`: [`Game::check_command`] and [`Game::preview_command`] try a command on a copy.
pub trait Game: Sized + Clone + Send + Debug + 'static {
    type Settings: GameSettings;
    type Command: Clone + Debug + PartialEq + Send + Sync + 'static;
    type WorldEvent: Clone + Debug + PartialEq + Send + Sync + 'static;
    type View: Debug;
    /// Held by the authority beside the game and never sent as it is. `()` for games that hide
    /// nothing.
    type Secret: GameSecret;

    const INFO: GameInfo;

    /// `settings` are already parsed ([`GameSettings::parse`]); only what depends on the
    /// players (seat count, seat kinds) is left to check here.
    fn new(settings: &Self::Settings, players: &[PlayerSpec]) -> Result<Self, SetupError>;

    fn clock(&self) -> Clock;

    /// Validates and applies atomically.
    fn apply_command(&mut self, slot: Slot, command: &Self::Command) -> Result<(), CommandError>;

    /// What [`Game::apply_command`] would answer, without changing anything: a client asks before
    /// offering an action, so it never copies a rule. A refused command leaves no trace, so trying
    /// it on a copy gives exactly that answer; a game may override this with a cheaper check.
    fn check_command(&self, slot: Slot, command: &Self::Command) -> Result<(), CommandError> {
        self.clone().apply_command(slot, command)
    }

    /// The state right after `command`, to show what an action leads to (whether giving up ends
    /// the game, and who wins then). Real-time games apply queued commands on the next step, so
    /// this shows the queue, not the step.
    fn preview_command(&self, slot: Slot, command: &Self::Command) -> Result<Self, CommandError> {
        let mut next = self.clone();
        next.apply_command(slot, command)?;
        Ok(next)
    }

    /// Called by the authority once per frame, after the frame's commands and before
    /// [`Game::roll_world`]: settles the secret (chooses it, answers what depends on it) and says
    /// what everyone may learn of it, as events. They are applied before `roll_world` runs.
    fn roll_secret(&self, _secret: &mut Self::Secret, _rng: &mut CoreRng) -> Vec<Self::WorldEvent> {
        Vec::new()
    }

    /// Called by the authority once per frame, after the frame's commands. May return nothing.
    fn roll_world(&self, rng: &mut CoreRng) -> Vec<Self::WorldEvent>;

    /// Called by the authority once, in the frame the game finishes: what was hidden, as events
    /// for everyone (replays included). `apply_world` must accept them on a finished game.
    fn reveal(&self, _secret: &Self::Secret) -> Vec<Self::WorldEvent> {
        Vec::new()
    }

    /// Whether `slot` still owes its private command (a fleet to place). Bots are asked for it
    /// when a frame is sealed; a host may give people a time limit and then
    /// [`Game::default_secret_command`].
    fn awaits_secret(&self, _slot: Slot) -> bool {
        false
    }

    /// A seat's private command: it changes only the secret and is never put in a frame. What
    /// the others may learn of it comes from [`Game::roll_secret`]. Validates and applies
    /// atomically, like [`Game::apply_command`].
    fn apply_secret_command(
        &self,
        _secret: &mut Self::Secret,
        _slot: Slot,
        _command: &SecretCommand<Self>,
    ) -> Result<(), CommandError> {
        Err(CommandError::Illegal("this game takes no private commands"))
    }

    /// The private command chosen for a seat that did not send one in time (a fleet placed at
    /// random).
    fn default_secret_command(
        &self,
        _slot: Slot,
        _rng: &mut CoreRng,
    ) -> Option<SecretCommand<Self>> {
        None
    }

    /// What `slot` may see of the secret (its own fleet).
    fn seat_secret(&self, _secret: &Self::Secret, _slot: Slot) -> Option<SeatSecret<Self>> {
        None
    }

    fn apply_world(&mut self, event: &Self::WorldEvent) -> Result<(), CommandError>;

    /// One tick of a real-time game. Not called for turn-based games or after the game ended.
    fn step(&mut self) {}

    /// The seat leaves play for good (left, kicked, disconnect budget exhausted).
    fn drop_player(&mut self, slot: Slot);

    fn awaiting(&self) -> Awaiting;

    fn status(&self) -> Status;

    fn view(&self, viewer: Viewer) -> Self::View;

    fn hash_state(&self, hasher: &mut StateHasher);

    fn state_hash(&self) -> u64 {
        let mut hasher = StateHasher::new();
        self.hash_state(&mut hasher);
        hasher.finish()
    }
}
