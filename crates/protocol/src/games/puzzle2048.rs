use mg_puzzle2048::{
    Cell, Command, Direction, MergeSource, Phase, Puzzle2048, Settings, SpawnValue, Tile,
    TileOrigin, View, WorldEvent,
};

use crate::proto;
use crate::wire_error::WireError;
use crate::wire_game::WireGame;

impl WireGame for Puzzle2048 {
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<Settings, WireError> {
        match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_settings::Game::Puzzle2048(settings) => Ok(Settings {
                width: settings.width,
                height: settings.height,
                target: settings.target,
            }),
            _ => Err(WireError::WrongGame),
        }
    }

    fn settings_to_wire(settings: &Settings) -> proto::GameSettings {
        proto::GameSettings {
            game: Some(proto::game_settings::Game::Puzzle2048(
                proto::Puzzle2048Settings {
                    width: settings.width,
                    height: settings.height,
                    target: settings.target,
                },
            )),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let proto::game_command::Game::Puzzle2048(command) =
            command.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            proto::puzzle2048_command::Command::Move(direction) => {
                Ok(Command::Move(direction_from_wire(*direction, "move")?))
            }
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let Command::Move(direction) = *command;
        proto::GameCommand {
            game: Some(proto::game_command::Game::Puzzle2048(
                proto::Puzzle2048Command {
                    command: Some(proto::puzzle2048_command::Command::Move(
                        direction_to_wire(direction).into(),
                    )),
                },
            )),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let proto::game_world_event::Game::Puzzle2048(event) =
            event.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            proto::puzzle2048_world_event::Event::TileSpawned(spawn) => {
                Ok(WorldEvent::TileSpawned {
                    cell: cell_from_wire(spawn.cell.as_ref())?,
                    value: spawn_value_from_wire(spawn.value)?,
                })
            }
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let WorldEvent::TileSpawned { cell, value } = *event;
        let value = match value {
            SpawnValue::Two => proto::Puzzle2048SpawnValue::Two,
            SpawnValue::Four => proto::Puzzle2048SpawnValue::Four,
        };
        proto::GameWorldEvent {
            game: Some(proto::game_world_event::Game::Puzzle2048(
                proto::Puzzle2048WorldEvent {
                    event: Some(proto::puzzle2048_world_event::Event::TileSpawned(
                        proto::Puzzle2048TileSpawned {
                            cell: Some(cell_to_wire(cell)),
                            value: value.into(),
                        },
                    )),
                },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        let phase = match view.phase {
            Phase::Playing => proto::Puzzle2048Phase::Playing,
            Phase::Won => proto::Puzzle2048Phase::Won,
            Phase::Lost => proto::Puzzle2048Phase::Lost,
            Phase::Abandoned => proto::Puzzle2048Phase::Abandoned,
        };
        let last_move = view
            .last_move
            .map_or(proto::Puzzle2048Direction::Unspecified, direction_to_wire);
        proto::GameView {
            game: Some(proto::game_view::Game::Puzzle2048(proto::Puzzle2048View {
                width: u32::from(view.width),
                height: u32::from(view.height),
                target: view.target.get(),
                tiles: view.tiles.iter().map(tile_to_wire).collect(),
                // Unreachable saturation: see the field comment in puzzle2048.proto.
                score: u32::try_from(view.score.get()).unwrap_or(u32::MAX),
                moves: view.moves,
                highest_tile: view.highest_tile.map_or(0, |tile| tile.get()),
                last_move: last_move.into(),
                phase: phase.into(),
                movable_directions: view
                    .movable_directions
                    .iter()
                    .map(|&direction| direction_to_wire(direction).into())
                    .collect(),
            })),
        }
    }
}

fn direction_from_wire(raw: i32, field: &'static str) -> Result<Direction, WireError> {
    match proto::Puzzle2048Direction::try_from(raw) {
        Ok(proto::Puzzle2048Direction::Up) => Ok(Direction::Up),
        Ok(proto::Puzzle2048Direction::Down) => Ok(Direction::Down),
        Ok(proto::Puzzle2048Direction::Left) => Ok(Direction::Left),
        Ok(proto::Puzzle2048Direction::Right) => Ok(Direction::Right),
        Ok(proto::Puzzle2048Direction::Unspecified) => Err(WireError::Missing(field)),
        Err(_) => Err(WireError::UnknownEnumValue(field)),
    }
}

fn direction_to_wire(direction: Direction) -> proto::Puzzle2048Direction {
    match direction {
        Direction::Up => proto::Puzzle2048Direction::Up,
        Direction::Down => proto::Puzzle2048Direction::Down,
        Direction::Left => proto::Puzzle2048Direction::Left,
        Direction::Right => proto::Puzzle2048Direction::Right,
    }
}

fn spawn_value_from_wire(raw: i32) -> Result<SpawnValue, WireError> {
    match proto::Puzzle2048SpawnValue::try_from(raw) {
        Ok(proto::Puzzle2048SpawnValue::Two) => Ok(SpawnValue::Two),
        Ok(proto::Puzzle2048SpawnValue::Four) => Ok(SpawnValue::Four),
        Ok(proto::Puzzle2048SpawnValue::Unspecified) => Err(WireError::Missing("value")),
        Err(_) => Err(WireError::UnknownEnumValue("value")),
    }
}

/// Only checks that the coordinates fit the domain type; whether the cell is on the board is a
/// game rule (`apply_world` rejects it).
fn cell_from_wire(cell: Option<&proto::Puzzle2048Cell>) -> Result<Cell, WireError> {
    let cell = cell.ok_or(WireError::Missing("cell"))?;
    let row = u8::try_from(cell.row).map_err(|source| WireError::IntegerOutOfRange {
        field: "cell.row",
        source,
    })?;
    let col = u8::try_from(cell.col).map_err(|source| WireError::IntegerOutOfRange {
        field: "cell.col",
        source,
    })?;
    Ok(Cell::new(row, col))
}

fn cell_to_wire(cell: Cell) -> proto::Puzzle2048Cell {
    proto::Puzzle2048Cell {
        row: u32::from(cell.row),
        col: u32::from(cell.col),
    }
}

fn merge_source_to_wire(source: &MergeSource) -> proto::Puzzle2048MergeSource {
    proto::Puzzle2048MergeSource {
        id: source.id.get(),
        from: Some(cell_to_wire(source.from)),
    }
}

fn tile_to_wire(tile: &Tile) -> proto::Puzzle2048Tile {
    let origin = match &tile.origin {
        TileOrigin::Slid { from } => {
            proto::puzzle2048_tile::Origin::Slid(proto::Puzzle2048SlidOrigin {
                from: Some(cell_to_wire(*from)),
            })
        }
        TileOrigin::Merged { sources } => {
            proto::puzzle2048_tile::Origin::Merged(proto::Puzzle2048MergedOrigin {
                sources: sources.iter().map(merge_source_to_wire).collect(),
            })
        }
        TileOrigin::Spawned => {
            proto::puzzle2048_tile::Origin::Spawned(proto::Puzzle2048SpawnedOrigin {})
        }
    };
    proto::Puzzle2048Tile {
        id: tile.id.get(),
        value: tile.value.get(),
        cell: Some(cell_to_wire(tile.cell)),
        origin: Some(origin),
    }
}

#[cfg(test)]
#[path = "puzzle2048_tests.rs"]
mod tests;
