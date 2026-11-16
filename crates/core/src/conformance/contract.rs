use super::setup::Setup;
use crate::bot_seat::BotSeat;
use crate::game::Game;
use crate::player_spec::PlayerSpec;
use crate::rng::CoreRng;
use crate::slot::Slot;

pub trait Conformance: Game {
    /// Cover edge sizes, every mode and bot seats.
    fn sample_setups() -> Vec<Setup<Self>>;

    /// Must all be rejected, by `GameSettings::parse` (out-of-range values, huge sizes, broken
    /// rules between fields) or by `Game::new` (wrong seat counts).
    fn invalid_setups() -> Vec<Setup<Self>>;

    /// May be illegal; bias towards plausible moves so simulated games actually progress.
    fn arbitrary_command(game: &Self, slot: Slot, rng: &mut CoreRng) -> Self::Command;

    fn bots_for(_players: &[PlayerSpec]) -> Vec<BotSeat<Self>> {
        Vec::new()
    }

    /// Checked after every frame.
    fn check_invariants(&self) -> Result<(), String> {
        Ok(())
    }

    fn max_frames() -> u64 {
        3_000
    }
}
