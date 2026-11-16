use mg_core::{CommandError, CoreRng, Game, GameSettings, PlayerSpec, Slot, Viewer};

use crate::{
    Cell, Command, Direction, Puzzle2048, RawSettings, Settings, SpawnValue, View, WorldEvent,
};

pub(super) const PLAYER: Slot = Slot::new(0);

pub(super) fn raw_settings(width: u32, height: u32, target: u32) -> RawSettings {
    RawSettings {
        width,
        height,
        target,
    }
}

pub(super) fn settings(width: u32, height: u32, target: u32) -> Settings {
    Settings::parse(raw_settings(width, height, target)).unwrap()
}

/// A running game with these row-major values (0 = empty), no spawn due; the target is 2048, or
/// the largest tile a smaller board can hold.
pub(super) fn board(width: u32, height: u32, values: &[u32]) -> Puzzle2048 {
    let largest_tile = 1_u32 << (width * height + 1).min(31);
    Puzzle2048::with_values(&settings(width, height, largest_tile.min(2048)), values)
}

/// A new game after the authority delivered the initial spawns.
pub(super) fn started(settings: &Settings, rng: &mut CoreRng) -> Puzzle2048 {
    let mut game = Puzzle2048::new(settings, &[PlayerSpec::HUMAN]).unwrap();
    deliver_spawns(&mut game, rng);
    game
}

pub(super) fn view(game: &Puzzle2048) -> View {
    game.view(Viewer::Spectator)
}

pub(super) fn values(game: &Puzzle2048) -> Vec<u32> {
    view(game).cell_values()
}

pub(super) fn tile_count(game: &Puzzle2048) -> usize {
    view(game).tiles.len()
}

pub(super) fn try_move(game: &mut Puzzle2048, direction: Direction) -> Result<(), CommandError> {
    game.apply_command(PLAYER, &Command::Move(direction))
}

pub(super) fn spawn(
    game: &mut Puzzle2048,
    row: u8,
    col: u8,
    value: SpawnValue,
) -> Result<(), CommandError> {
    game.apply_world(&WorldEvent::TileSpawned {
        cell: Cell::new(row, col),
        value,
    })
}

pub(super) fn deliver_spawns(game: &mut Puzzle2048, rng: &mut CoreRng) {
    for event in game.roll_world(rng) {
        game.apply_world(&event).unwrap();
    }
}

/// What the authority does for one turn: the move, then its spawn.
pub(super) fn play(
    game: &mut Puzzle2048,
    direction: Direction,
    rng: &mut CoreRng,
) -> Result<(), CommandError> {
    try_move(game, direction)?;
    deliver_spawns(game, rng);
    Ok(())
}
