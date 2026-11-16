use mg_core::PlayerKind;
use mg_snake::{
    BotKind, Command, DeadSnakeBehavior, DeathReason, Direction, GameEnd, Point, RawSettings,
    Snake, SnakeView, View, WallCollisionMode, WorldEvent,
};

use crate::proto;
use crate::slot_to_wire;
use crate::wire_error::WireError;
use crate::wire_game::{WireBots, WireGame};

impl WireGame for Snake {
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<RawSettings, WireError> {
        let settings = match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_settings::Game::Snake(settings) => settings,
            _ => return Err(WireError::WrongGame),
        };
        Ok(RawSettings {
            field_width: settings.field_width,
            field_height: settings.field_height,
            wall_collision_mode: wall_collision_mode_from_wire(settings.wall_collision_mode)?,
            dead_snake_behavior: dead_snake_behavior_from_wire(settings.dead_snake_behavior)?,
            max_food_count: settings.max_food_count,
            food_spawn_permille: settings.food_spawn_permille,
            tick_interval_ms: settings.tick_interval_ms,
        })
    }

    fn settings_to_wire(settings: &RawSettings) -> proto::GameSettings {
        proto::GameSettings {
            game: Some(proto::game_settings::Game::Snake(proto::SnakeSettings {
                field_width: settings.field_width,
                field_height: settings.field_height,
                wall_collision_mode: wall_collision_mode_to_wire(settings.wall_collision_mode)
                    .into(),
                dead_snake_behavior: dead_snake_behavior_to_wire(settings.dead_snake_behavior)
                    .into(),
                max_food_count: settings.max_food_count,
                food_spawn_permille: settings.food_spawn_permille,
                tick_interval_ms: settings.tick_interval_ms,
            })),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let command = match command.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_command::Game::Snake(command) => command,
            _ => return Err(WireError::WrongGame),
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            proto::snake_command::Command::Turn(turn) => {
                Ok(Command::Turn(direction_from_wire(turn.direction)?))
            }
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let command = match *command {
            Command::Turn(direction) => proto::snake_command::Command::Turn(proto::SnakeTurn {
                direction: direction_to_wire(direction).into(),
            }),
        };
        proto::GameCommand {
            game: Some(proto::game_command::Game::Snake(proto::SnakeCommand {
                command: Some(command),
            })),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let event = match event.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_world_event::Game::Snake(event) => event,
            _ => return Err(WireError::WrongGame),
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            proto::snake_world_event::Event::SpawnFood(spawn) => {
                let cell = spawn.cell.as_ref().ok_or(WireError::Missing("cell"))?;
                Ok(WorldEvent::SpawnFood(point_from_wire(cell)?))
            }
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let event = match *event {
            WorldEvent::SpawnFood(cell) => {
                proto::snake_world_event::Event::SpawnFood(proto::SnakeSpawnFood {
                    cell: Some(point_to_wire(cell)),
                })
            }
        };
        proto::GameWorldEvent {
            game: Some(proto::game_world_event::Game::Snake(
                proto::SnakeWorldEvent { event: Some(event) },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        proto::GameView {
            game: Some(proto::game_view::Game::Snake(proto::SnakeView {
                tick: view.steps.get(),
                field_width: u32::from(view.field_width),
                field_height: u32::from(view.field_height),
                tick_interval_ms: view.tick_interval_ms,
                wall_collision_mode: wall_collision_mode_to_wire(view.wall_collision_mode).into(),
                dead_snake_behavior: dead_snake_behavior_to_wire(view.dead_snake_behavior).into(),
                max_food_count: u32::from(view.max_food_count),
                snakes: view.snakes.iter().map(seat_view_to_wire).collect(),
                food: view.food.iter().copied().map(point_to_wire).collect(),
                you: view.you.map(slot_to_wire),
                end: view.end.as_ref().map(game_end_to_wire),
            })),
        }
    }
}

impl WireBots for Snake {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<BotKind, WireError> {
        let kind = match kind.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::bot_kind::Game::Snake(kind) => *kind,
            _ => return Err(WireError::WrongGame),
        };
        match proto::SnakeBotKind::try_from(kind) {
            Ok(proto::SnakeBotKind::Efficient) => Ok(BotKind::Efficient),
            Ok(proto::SnakeBotKind::Random) => Ok(BotKind::Random),
            Ok(proto::SnakeBotKind::Unspecified) => Err(WireError::Missing("bot_kind")),
            Err(_) => Err(WireError::UnknownEnumValue("bot_kind")),
        }
    }

    fn bot_kind_to_wire(kind: BotKind) -> proto::BotKind {
        let kind = match kind {
            BotKind::Efficient => proto::SnakeBotKind::Efficient,
            BotKind::Random => proto::SnakeBotKind::Random,
        };
        proto::BotKind {
            game: Some(proto::bot_kind::Game::Snake(kind.into())),
        }
    }
}

fn point_from_wire(point: &proto::SnakePoint) -> Result<Point, WireError> {
    let x = u16::try_from(point.x)
        .map_err(|source| WireError::IntegerOutOfRange { field: "x", source })?;
    let y = u16::try_from(point.y)
        .map_err(|source| WireError::IntegerOutOfRange { field: "y", source })?;
    Ok(Point::new(x, y))
}

fn point_to_wire(point: Point) -> proto::SnakePoint {
    proto::SnakePoint {
        x: u32::from(point.x),
        y: u32::from(point.y),
    }
}

fn direction_from_wire(direction: i32) -> Result<Direction, WireError> {
    match proto::SnakeDirection::try_from(direction) {
        Ok(proto::SnakeDirection::Up) => Ok(Direction::Up),
        Ok(proto::SnakeDirection::Down) => Ok(Direction::Down),
        Ok(proto::SnakeDirection::Left) => Ok(Direction::Left),
        Ok(proto::SnakeDirection::Right) => Ok(Direction::Right),
        Ok(proto::SnakeDirection::Unspecified) => Err(WireError::Missing("direction")),
        Err(_) => Err(WireError::UnknownEnumValue("direction")),
    }
}

fn direction_to_wire(direction: Direction) -> proto::SnakeDirection {
    match direction {
        Direction::Up => proto::SnakeDirection::Up,
        Direction::Down => proto::SnakeDirection::Down,
        Direction::Left => proto::SnakeDirection::Left,
        Direction::Right => proto::SnakeDirection::Right,
    }
}

fn wall_collision_mode_from_wire(mode: i32) -> Result<WallCollisionMode, WireError> {
    match proto::SnakeWallCollisionMode::try_from(mode) {
        Ok(proto::SnakeWallCollisionMode::Death) => Ok(WallCollisionMode::Death),
        Ok(proto::SnakeWallCollisionMode::WrapAround) => Ok(WallCollisionMode::WrapAround),
        Ok(proto::SnakeWallCollisionMode::Unspecified) => {
            Err(WireError::Missing("wall_collision_mode"))
        }
        Err(_) => Err(WireError::UnknownEnumValue("wall_collision_mode")),
    }
}

fn wall_collision_mode_to_wire(mode: WallCollisionMode) -> proto::SnakeWallCollisionMode {
    match mode {
        WallCollisionMode::Death => proto::SnakeWallCollisionMode::Death,
        WallCollisionMode::WrapAround => proto::SnakeWallCollisionMode::WrapAround,
    }
}

fn dead_snake_behavior_from_wire(behavior: i32) -> Result<DeadSnakeBehavior, WireError> {
    match proto::SnakeDeadSnakeBehavior::try_from(behavior) {
        Ok(proto::SnakeDeadSnakeBehavior::Disappear) => Ok(DeadSnakeBehavior::Disappear),
        Ok(proto::SnakeDeadSnakeBehavior::StayOnField) => Ok(DeadSnakeBehavior::StayOnField),
        Ok(proto::SnakeDeadSnakeBehavior::Unspecified) => {
            Err(WireError::Missing("dead_snake_behavior"))
        }
        Err(_) => Err(WireError::UnknownEnumValue("dead_snake_behavior")),
    }
}

fn dead_snake_behavior_to_wire(behavior: DeadSnakeBehavior) -> proto::SnakeDeadSnakeBehavior {
    match behavior {
        DeadSnakeBehavior::Disappear => proto::SnakeDeadSnakeBehavior::Disappear,
        DeadSnakeBehavior::StayOnField => proto::SnakeDeadSnakeBehavior::StayOnField,
    }
}

fn death_reason_to_wire(reason: DeathReason) -> proto::SnakeDeathReason {
    match reason {
        DeathReason::WallCollision => proto::SnakeDeathReason::WallCollision,
        DeathReason::SelfCollision => proto::SnakeDeathReason::SelfCollision,
        DeathReason::SnakeCollision => proto::SnakeDeathReason::SnakeCollision,
        DeathReason::PlayerDisconnected => proto::SnakeDeathReason::PlayerDisconnected,
    }
}

fn seat_view_to_wire(snake: &SnakeView) -> proto::SnakeSeatView {
    proto::SnakeSeatView {
        slot: slot_to_wire(snake.slot),
        is_bot: snake.player == PlayerKind::Bot,
        body: snake.body.iter().copied().map(point_to_wire).collect(),
        direction: direction_to_wire(snake.direction).into(),
        queued_turns: snake
            .queued_turns
            .iter()
            .map(|&turn| direction_to_wire(turn).into())
            .collect(),
        alive: snake.is_alive(),
        death_reason: snake
            .death
            .map_or(proto::SnakeDeathReason::Unspecified, death_reason_to_wire)
            .into(),
        score: snake.score.get(),
    }
}

fn game_end_to_wire(end: &GameEnd) -> proto::SnakeGameEnd {
    proto::SnakeGameEnd {
        reason: death_reason_to_wire(end.reason).into(),
        winner_slots: end.winners.iter().copied().map(slot_to_wire).collect(),
    }
}

#[cfg(test)]
#[path = "snake_tests.rs"]
mod tests;
