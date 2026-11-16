use mg_core::CoreRng;

/// Seed of game `index` of an arena run; independent of which thread plays the game.
pub fn game_seed(base_seed: u64, index: u32) -> u64 {
    CoreRng::derive(base_seed, u64::from(index)).next_u64()
}
