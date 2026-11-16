use mg_stack_attack::{
    CargoBox, Cell, Column, Command, Crane, CraneState, Direction, EndReason, GameEvent, PatternId,
    RawSettings, Side, StackAttack, View, WorkerState, WorkerStatus, WorkerView, WorldEvent,
};

use crate::proto;
use crate::proto::{game_command, game_settings, game_view, game_world_event};
use crate::proto::{stack_attack_command, stack_attack_event, stack_attack_world_event};
use crate::slot_to_wire;
use crate::wire_error::WireError;
use crate::wire_game::WireGame;

impl WireGame for StackAttack {
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<RawSettings, WireError> {
        match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            game_settings::Game::StackAttack(settings) => Ok(RawSettings {
                field_width: settings.field_width,
                field_height: settings.field_height,
            }),
            _ => Err(WireError::WrongGame),
        }
    }

    fn settings_to_wire(settings: &RawSettings) -> proto::GameSettings {
        proto::GameSettings {
            game: Some(game_settings::Game::StackAttack(
                proto::StackAttackSettings {
                    field_width: settings.field_width,
                    field_height: settings.field_height,
                },
            )),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let game_command::Game::StackAttack(command) =
            command.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            stack_attack_command::Command::Move(walk) => {
                Ok(Command::Move(direction_from_wire(walk.direction)?))
            }
            stack_attack_command::Command::Jump(_) => Ok(Command::Jump),
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let command = match command {
            Command::Move(direction) => {
                stack_attack_command::Command::Move(proto::StackAttackMove {
                    direction: direction_to_wire(*direction) as i32,
                })
            }
            Command::Jump => stack_attack_command::Command::Jump(proto::StackAttackJump {}),
        };
        proto::GameCommand {
            game: Some(game_command::Game::StackAttack(proto::StackAttackCommand {
                command: Some(command),
            })),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let game_world_event::Game::StackAttack(event) =
            event.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            stack_attack_world_event::Event::CraneSpawn(spawn) => Ok(WorldEvent::CraneSpawned {
                target_x: Column::new(spawn.target_x),
                from: side_from_wire(spawn.from_side)?,
                pattern: pattern_from_wire(spawn.pattern_id)?,
            }),
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let WorldEvent::CraneSpawned {
            target_x,
            from,
            pattern,
        } = *event;
        let spawn = proto::StackAttackCraneSpawn {
            target_x: target_x.get(),
            from_side: side_to_wire(from) as i32,
            pattern_id: u32::from(pattern.get()),
        };
        proto::GameWorldEvent {
            game: Some(game_world_event::Game::StackAttack(
                proto::StackAttackWorldEvent {
                    event: Some(stack_attack_world_event::Event::CraneSpawn(spawn)),
                },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        let wire = proto::StackAttackView {
            you: view.you.map(slot_to_wire),
            field_width: view.field_width,
            field_height: view.field_height,
            tick_ms: view.tick_ms,
            workers: view.workers.iter().map(worker_to_wire).collect(),
            boxes: view.boxes.iter().map(box_to_wire).collect(),
            cranes: view.cranes.iter().map(crane_to_wire).collect(),
            score: view.score.get(),
            lines_cleared: view.lines_cleared,
            boxes_pushed: view.boxes_pushed,
            level: view.level.get(),
            ticks_survived: view.ticks_survived.get(),
            events: view.events.iter().map(event_to_wire).collect(),
            end_reason: view
                .end_reason
                .map(|reason| end_reason_to_wire(reason) as i32),
        };
        proto::GameView {
            game: Some(game_view::Game::StackAttack(wire)),
        }
    }
}

fn direction_from_wire(value: i32) -> Result<Direction, WireError> {
    match proto::StackAttackDirection::try_from(value) {
        Ok(proto::StackAttackDirection::Left) => Ok(Direction::Left),
        Ok(proto::StackAttackDirection::Right) => Ok(Direction::Right),
        Ok(proto::StackAttackDirection::Unspecified) => Err(WireError::Missing("direction")),
        Err(_) => Err(WireError::UnknownEnumValue("direction")),
    }
}

fn direction_to_wire(direction: Direction) -> proto::StackAttackDirection {
    match direction {
        Direction::Left => proto::StackAttackDirection::Left,
        Direction::Right => proto::StackAttackDirection::Right,
    }
}

fn side_from_wire(value: i32) -> Result<Side, WireError> {
    match proto::StackAttackSide::try_from(value) {
        Ok(proto::StackAttackSide::Left) => Ok(Side::Left),
        Ok(proto::StackAttackSide::Right) => Ok(Side::Right),
        Ok(proto::StackAttackSide::Unspecified) => Err(WireError::Missing("from_side")),
        Err(_) => Err(WireError::UnknownEnumValue("from_side")),
    }
}

fn side_to_wire(side: Side) -> proto::StackAttackSide {
    match side {
        Side::Left => proto::StackAttackSide::Left,
        Side::Right => proto::StackAttackSide::Right,
    }
}

fn pattern_from_wire(value: u32) -> Result<PatternId, WireError> {
    let value = u8::try_from(value).map_err(|source| WireError::IntegerOutOfRange {
        field: "pattern_id",
        source,
    })?;
    PatternId::new(value).ok_or(WireError::OutOfRange("pattern_id"))
}

fn cell_to_wire(cell: Cell) -> proto::StackAttackCell {
    proto::StackAttackCell {
        x: cell.x,
        y: cell.y,
    }
}

fn worker_to_wire(worker: &WorkerView) -> proto::StackAttackWorker {
    let state = match worker.state {
        WorkerState::Grounded => proto::StackAttackWorkerState::Grounded,
        WorkerState::Jumping => proto::StackAttackWorkerState::Jumping,
        WorkerState::Falling => proto::StackAttackWorkerState::Falling,
    };
    let status = match worker.status {
        WorkerStatus::Alive => proto::StackAttackWorkerStatus::Alive,
        WorkerStatus::Crushed => proto::StackAttackWorkerStatus::Crushed,
        WorkerStatus::Left => proto::StackAttackWorkerStatus::Left,
    };
    proto::StackAttackWorker {
        slot: slot_to_wire(worker.slot),
        feet: Some(cell_to_wire(worker.feet)),
        state: state as i32,
        status: status as i32,
        queued_commands: u32::try_from(worker.queued_commands).unwrap_or(u32::MAX),
    }
}

fn box_to_wire(cargo: &CargoBox) -> proto::StackAttackBox {
    proto::StackAttackBox {
        id: cargo.id.get(),
        cell: Some(cell_to_wire(cargo.cell)),
        pattern_id: u32::from(cargo.pattern.get()),
        falling: cargo.falling,
    }
}

fn crane_to_wire(crane: &Crane) -> proto::StackAttackCrane {
    let state = match crane.state {
        CraneState::Moving => proto::StackAttackCraneState::Moving,
        CraneState::Dropping => proto::StackAttackCraneState::Dropping,
        CraneState::Leaving => proto::StackAttackCraneState::Leaving,
    };
    proto::StackAttackCrane {
        id: crane.id.get(),
        x: crane.x.get(),
        target_x: crane.target_x.get(),
        state: state as i32,
        pattern_id: u32::from(crane.pattern.get()),
    }
}

fn end_reason_to_wire(reason: EndReason) -> proto::StackAttackEndReason {
    match reason {
        EndReason::WorkersCrushed => proto::StackAttackEndReason::WorkersCrushed,
        EndReason::CeilingReached => proto::StackAttackEndReason::CeilingReached,
        EndReason::AllWorkersLeft => proto::StackAttackEndReason::AllWorkersLeft,
    }
}

fn event_to_wire(event: &GameEvent) -> proto::StackAttackEvent {
    use stack_attack_event::Event;
    let event = match event {
        GameEvent::CraneSpawned {
            crane_id,
            start_x,
            target_x,
        } => Event::CraneSpawned(proto::StackAttackCraneSpawnedEvent {
            crane_id: crane_id.get(),
            start_x: start_x.get(),
            target_x: target_x.get(),
        }),
        GameEvent::BoxDropped {
            crane_id,
            box_id,
            x,
        } => Event::BoxDropped(proto::StackAttackBoxDroppedEvent {
            crane_id: crane_id.get(),
            box_id: box_id.get(),
            x: x.get(),
        }),
        GameEvent::BoxLanded { box_id, cell } => {
            Event::BoxLanded(proto::StackAttackBoxLandedEvent {
                box_id: box_id.get(),
                cell: Some(cell_to_wire(*cell)),
            })
        }
        GameEvent::BoxPushed {
            box_id,
            from_x,
            to_x,
            by,
        } => Event::BoxPushed(proto::StackAttackBoxPushedEvent {
            box_id: box_id.get(),
            from_x: from_x.get(),
            to_x: to_x.get(),
            by_slot: slot_to_wire(*by),
        }),
        GameEvent::WorkerJumped { slot } => {
            Event::WorkerJumped(proto::StackAttackWorkerJumpedEvent {
                slot: slot_to_wire(*slot),
            })
        }
        GameEvent::WorkerLanded { slot, feet } => {
            Event::WorkerLanded(proto::StackAttackWorkerLandedEvent {
                slot: slot_to_wire(*slot),
                feet: Some(cell_to_wire(*feet)),
            })
        }
        GameEvent::WorkerCrushed { slot, box_id } => {
            Event::WorkerCrushed(proto::StackAttackWorkerCrushedEvent {
                slot: slot_to_wire(*slot),
                box_id: box_id.get(),
            })
        }
        GameEvent::WorkerLeft { slot } => Event::WorkerLeft(proto::StackAttackWorkerLeftEvent {
            slot: slot_to_wire(*slot),
        }),
        GameEvent::LinesCleared { rows, points } => {
            Event::LinesCleared(proto::StackAttackLinesClearedEvent {
                rows: rows.iter().map(|row| row.get()).collect(),
                points: *points,
            })
        }
    };
    proto::StackAttackEvent { event: Some(event) }
}

#[cfg(test)]
#[path = "stack_attack_tests.rs"]
mod tests;
