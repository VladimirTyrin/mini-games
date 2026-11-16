use std::fmt::Debug;

use crate::awaiting::Awaiting;
use crate::clock::Clock;
use crate::command_error::CommandError;
use crate::game_info::GameInfo;
use crate::hash::StateHasher;
use crate::outcome::Status;
use crate::player::PlayerSpec;
use crate::rng::CoreRng;
use crate::setup_error::SetupError;
use crate::slot::Slot;
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
pub trait Game: Sized + Send + 'static {
    type Settings: Clone + Debug + Send + Sync + 'static;
    type Command: Clone + Debug + PartialEq + Send + Sync + 'static;
    type WorldEvent: Clone + Debug + PartialEq + Send + Sync + 'static;
    type View: Debug;

    const INFO: GameInfo;

    /// Settings come from untrusted clients and replay files: validate everything before
    /// allocating anything sized by them.
    fn new(settings: &Self::Settings, players: &[PlayerSpec]) -> Result<Self, SetupError>;

    fn clock(&self) -> Clock;

    /// Validates and applies atomically.
    fn apply_command(&mut self, slot: Slot, command: &Self::Command) -> Result<(), CommandError>;

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
