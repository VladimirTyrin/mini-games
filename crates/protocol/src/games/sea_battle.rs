use mg_sea_battle::{
    BotKind, Cell, CellMark, Command, FLEET_LENGTHS, Fleet, LastShot, Orientation, Phase,
    RawSettings, SeaBattle, SecretCommand, Ship, ShotOutcome, View, Waters, WorldEvent,
};

use crate::proto::{
    game_command, game_seat_secret, game_secret_command, game_settings, game_view,
    game_world_event, sea_battle_command, sea_battle_secret_command, sea_battle_world_event,
};
use crate::{WireBots, WireError, WireGame, proto, slot_from_wire, slot_to_wire};

impl WireGame for SeaBattle {
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<RawSettings, WireError> {
        match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            game_settings::Game::SeaBattle(_) => Ok(RawSettings),
            _ => Err(WireError::WrongGame),
        }
    }

    fn settings_to_wire(_settings: &RawSettings) -> proto::GameSettings {
        proto::GameSettings {
            game: Some(game_settings::Game::SeaBattle(proto::SeaBattleSettings {})),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let game_command::Game::SeaBattle(command) =
            command.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            sea_battle_command::Command::Fire(fire) => {
                Ok(Command::Fire(cell_from_wire(fire.cell.as_ref())?))
            }
            sea_battle_command::Command::GiveUp(_) => Ok(Command::GiveUp),
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let command = match command {
            Command::Fire(cell) => sea_battle_command::Command::Fire(proto::SeaBattleFire {
                cell: Some(cell_to_wire(*cell)),
            }),
            Command::GiveUp => sea_battle_command::Command::GiveUp(proto::SeaBattleGiveUp {}),
        };
        proto::GameCommand {
            game: Some(game_command::Game::SeaBattle(proto::SeaBattleCommand {
                command: Some(command),
            })),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let game_world_event::Game::SeaBattle(event) =
            event.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            sea_battle_world_event::Event::FleetPlaced(placed) => {
                Ok(WorldEvent::FleetPlaced(slot_from_wire(placed.slot)?))
            }
            sea_battle_world_event::Event::FirstShooterChosen(chosen) => {
                Ok(WorldEvent::FirstShooterChosen(slot_from_wire(chosen.slot)?))
            }
            sea_battle_world_event::Event::ShotResolved(resolved) => {
                let outcome = match proto::SeaBattleShotOutcome::try_from(resolved.outcome) {
                    Ok(proto::SeaBattleShotOutcome::Miss) => ShotOutcome::Miss,
                    Ok(proto::SeaBattleShotOutcome::Hit) => ShotOutcome::Hit,
                    Ok(proto::SeaBattleShotOutcome::Sunk) => {
                        let sunk = resolved.sunk.as_ref().ok_or(WireError::Missing("sunk"))?;
                        ShotOutcome::Sunk(ship_from_wire(sunk)?)
                    }
                    Ok(proto::SeaBattleShotOutcome::Unspecified) => {
                        return Err(WireError::Missing("outcome"));
                    }
                    Err(_) => return Err(WireError::UnknownEnumValue("outcome")),
                };
                Ok(WorldEvent::ShotResolved(outcome))
            }
            sea_battle_world_event::Event::FleetRevealed(revealed) => {
                Ok(WorldEvent::FleetRevealed {
                    slot: slot_from_wire(revealed.slot)?,
                    ships: ships_from_wire(&revealed.ships)?,
                })
            }
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let event = match event {
            WorldEvent::FleetPlaced(slot) => {
                sea_battle_world_event::Event::FleetPlaced(proto::SeaBattleFleetPlaced {
                    slot: slot_to_wire(*slot),
                })
            }
            WorldEvent::FirstShooterChosen(slot) => {
                sea_battle_world_event::Event::FirstShooterChosen(
                    proto::SeaBattleFirstShooterChosen {
                        slot: slot_to_wire(*slot),
                    },
                )
            }
            WorldEvent::ShotResolved(outcome) => {
                sea_battle_world_event::Event::ShotResolved(proto::SeaBattleShotResolved {
                    outcome: outcome_to_wire(*outcome).into(),
                    sunk: match outcome {
                        ShotOutcome::Sunk(ship) => Some(ship_to_wire(*ship)),
                        ShotOutcome::Miss | ShotOutcome::Hit => None,
                    },
                })
            }
            WorldEvent::FleetRevealed { slot, ships } => {
                sea_battle_world_event::Event::FleetRevealed(proto::SeaBattleFleetRevealed {
                    slot: slot_to_wire(*slot),
                    ships: ships.iter().copied().map(ship_to_wire).collect(),
                })
            }
        };
        proto::GameWorldEvent {
            game: Some(game_world_event::Game::SeaBattle(
                proto::SeaBattleWorldEvent { event: Some(event) },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        proto::GameView {
            game: Some(game_view::Game::SeaBattle(proto::SeaBattleView {
                phase: phase_to_wire(view.phase).into(),
                ready: view.ready.clone(),
                waters: view.waters.iter().map(waters_to_wire).collect(),
                turn_slot: view.turn.map(slot_to_wire),
                winner_slot: view.winner.map(slot_to_wire),
                last_shot: view.last_shot.map(last_shot_to_wire),
                revealed: view
                    .revealed
                    .iter()
                    .map(|fleet| proto::SeaBattleRevealedFleet {
                        ships: fleet.iter().flatten().copied().map(ship_to_wire).collect(),
                    })
                    .collect(),
            })),
        }
    }

    fn secret_command_from_wire(
        command: &proto::GameSecretCommand,
    ) -> Result<SecretCommand, WireError> {
        let game_secret_command::Game::SeaBattle(command) =
            command.game.as_ref().ok_or(WireError::Missing("game"))?;
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            sea_battle_secret_command::Command::PlaceFleet(fleet) => {
                Ok(SecretCommand::PlaceFleet(ships_from_wire(&fleet.ships)?))
            }
        }
    }

    fn secret_command_to_wire(command: &SecretCommand) -> proto::GameSecretCommand {
        let SecretCommand::PlaceFleet(ships) = command;
        proto::GameSecretCommand {
            game: Some(game_secret_command::Game::SeaBattle(
                proto::SeaBattleSecretCommand {
                    command: Some(sea_battle_secret_command::Command::PlaceFleet(
                        proto::SeaBattlePlaceFleet {
                            ships: ships.iter().copied().map(ship_to_wire).collect(),
                        },
                    )),
                },
            )),
        }
    }

    fn seat_secret_to_wire(fleet: &Fleet) -> proto::GameSeatSecret {
        proto::GameSeatSecret {
            game: Some(game_seat_secret::Game::SeaBattle(
                proto::SeaBattleSeatSecret {
                    ships: fleet.ships().iter().copied().map(ship_to_wire).collect(),
                },
            )),
        }
    }
}

impl WireBots for SeaBattle {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<BotKind, WireError> {
        let kind = match kind.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::bot_kind::Game::SeaBattle(kind) => *kind,
            _ => return Err(WireError::WrongGame),
        };
        match proto::SeaBattleBotKind::try_from(kind) {
            Ok(proto::SeaBattleBotKind::Easy) => Ok(BotKind::Easy),
            Ok(proto::SeaBattleBotKind::Medium) => Ok(BotKind::Medium),
            Ok(proto::SeaBattleBotKind::Hard) => Ok(BotKind::Hard),
            Ok(proto::SeaBattleBotKind::Unspecified) => Err(WireError::Missing("bot_kind")),
            Err(_) => Err(WireError::UnknownEnumValue("bot_kind")),
        }
    }

    fn bot_kind_to_wire(kind: BotKind) -> proto::BotKind {
        let kind = match kind {
            BotKind::Easy => proto::SeaBattleBotKind::Easy,
            BotKind::Medium => proto::SeaBattleBotKind::Medium,
            BotKind::Hard => proto::SeaBattleBotKind::Hard,
        };
        proto::BotKind {
            game: Some(proto::bot_kind::Game::SeaBattle(kind.into())),
        }
    }
}

fn cell_from_wire(cell: Option<&proto::SeaBattleCell>) -> Result<Cell, WireError> {
    let cell = cell.ok_or(WireError::Missing("cell"))?;
    let x = u8::try_from(cell.x).map_err(|_| WireError::OutOfRange("x"))?;
    let y = u8::try_from(cell.y).map_err(|_| WireError::OutOfRange("y"))?;
    Cell::new(x, y).ok_or(WireError::OutOfRange("cell"))
}

fn cell_to_wire(cell: Cell) -> proto::SeaBattleCell {
    proto::SeaBattleCell {
        x: u32::from(cell.x()),
        y: u32::from(cell.y()),
    }
}

/// A ship on the board; whether ships make a fleet is the game's rule (`Fleet::parse`).
fn ship_from_wire(ship: &proto::SeaBattleShip) -> Result<Ship, WireError> {
    let bow = cell_from_wire(ship.bow.as_ref())?;
    let length = u8::try_from(ship.length).map_err(|_| WireError::OutOfRange("length"))?;
    let orientation = match proto::SeaBattleOrientation::try_from(ship.orientation) {
        Ok(proto::SeaBattleOrientation::Horizontal) => Orientation::Horizontal,
        Ok(proto::SeaBattleOrientation::Vertical) => Orientation::Vertical,
        Ok(proto::SeaBattleOrientation::Unspecified) => {
            return Err(WireError::Missing("orientation"));
        }
        Err(_) => return Err(WireError::UnknownEnumValue("orientation")),
    };
    Ship::new(bow, length, orientation).ok_or(WireError::OutOfRange("ship"))
}

/// At most a fleet's worth, checked before anything is built from them.
fn ships_from_wire(ships: &[proto::SeaBattleShip]) -> Result<Vec<Ship>, WireError> {
    if ships.len() > FLEET_LENGTHS.len() {
        return Err(WireError::OutOfRange("ships"));
    }
    ships.iter().map(ship_from_wire).collect()
}

fn ship_to_wire(ship: Ship) -> proto::SeaBattleShip {
    let orientation = match ship.orientation() {
        Orientation::Horizontal => proto::SeaBattleOrientation::Horizontal,
        Orientation::Vertical => proto::SeaBattleOrientation::Vertical,
    };
    proto::SeaBattleShip {
        bow: Some(cell_to_wire(ship.bow())),
        length: u32::from(ship.length()),
        orientation: orientation.into(),
    }
}

fn outcome_to_wire(outcome: ShotOutcome) -> proto::SeaBattleShotOutcome {
    match outcome {
        ShotOutcome::Miss => proto::SeaBattleShotOutcome::Miss,
        ShotOutcome::Hit => proto::SeaBattleShotOutcome::Hit,
        ShotOutcome::Sunk(_) => proto::SeaBattleShotOutcome::Sunk,
    }
}

fn phase_to_wire(phase: Phase) -> proto::SeaBattlePhase {
    match phase {
        Phase::Placing => proto::SeaBattlePhase::Placing,
        Phase::Battle => proto::SeaBattlePhase::Battle,
        Phase::Won => proto::SeaBattlePhase::Won,
        Phase::Forfeited => proto::SeaBattlePhase::Forfeited,
        Phase::Abandoned => proto::SeaBattlePhase::Abandoned,
    }
}

fn mark_to_wire(mark: CellMark) -> proto::SeaBattleCellMark {
    match mark {
        CellMark::Unknown => proto::SeaBattleCellMark::Unknown,
        CellMark::Miss => proto::SeaBattleCellMark::Miss,
        CellMark::Hit => proto::SeaBattleCellMark::Hit,
        CellMark::Sunk => proto::SeaBattleCellMark::Sunk,
        CellMark::Cleared => proto::SeaBattleCellMark::Cleared,
    }
}

fn waters_to_wire(waters: &Waters) -> proto::SeaBattleWaters {
    proto::SeaBattleWaters {
        marks: waters
            .marks()
            .iter()
            .map(|mark| mark_to_wire(*mark).into())
            .collect(),
        sunk: waters.sunk().iter().copied().map(ship_to_wire).collect(),
        afloat: waters.afloat().into_iter().map(u32::from).collect(),
    }
}

fn last_shot_to_wire(last: LastShot) -> proto::SeaBattleLastShot {
    proto::SeaBattleLastShot {
        by_slot: slot_to_wire(last.by),
        cell: Some(cell_to_wire(last.cell)),
        outcome: outcome_to_wire(last.outcome).into(),
    }
}

#[cfg(test)]
#[path = "sea_battle_tests.rs"]
mod tests;
