use std::fmt::Debug;

use crate::awaiting::Awaiting;
use crate::clock::Clock;
use crate::command_error::CommandError;
use crate::game_info::GameInfo;
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
/// * randomness exists only in [`Game::roll_world`], which runs on the authority; its result
///   travels as `WorldEvent`s, so clients never need the seed;
/// * a rejected command or world event leaves the state untouched;
/// * [`Game::hash_state`] covers every gameplay-relevant field in a fixed order.
///
/// `Clone`: [`Game::check_command`] and [`Game::preview_command`] try a command on a copy.
pub trait Game: Sized + Clone + Send + Debug + 'static {
    type Settings: GameSettings;
    type Command: Clone + Debug + PartialEq + Send + Sync + 'static;
    type WorldEvent: Clone + Debug + PartialEq + Send + Sync + 'static;
    type View: Debug;

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

    /// Called by the authority once per frame, after the frame's commands. May return nothing.
    fn roll_world(&self, rng: &mut CoreRng) -> Vec<Self::WorldEvent>;

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
