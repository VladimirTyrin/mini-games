use mg_core::{Authority, Game, Match, PlayerSpec, Slot};
use mg_sea_battle::{
    BotKind, Cell, Command, Orientation, SeaBattle, SecretCommand, Settings, Ship, ShotOutcome,
    WorldEvent,
};
use prost::Message;

use crate::proto;
use crate::wire_error::WireError;
use crate::wire_game::{WireBots, WireGame};

fn binary<M: Message + Default + PartialEq + std::fmt::Debug>(message: &M) -> M {
    let decoded = M::decode(message.encode_to_vec().as_slice()).unwrap();
    assert_eq!(&decoded, message);
    decoded
}

fn ship(x: u8, y: u8, length: u8, orientation: Orientation) -> Ship {
    Ship::new(Cell::new(x, y).unwrap(), length, orientation).unwrap()
}

fn classic_ships() -> Vec<Ship> {
    use Orientation::Horizontal;
    vec![
        ship(0, 0, 4, Horizontal),
        ship(5, 0, 3, Horizontal),
        ship(0, 2, 3, Horizontal),
        ship(4, 2, 2, Horizontal),
        ship(7, 2, 2, Horizontal),
        ship(0, 4, 2, Horizontal),
        ship(3, 4, 1, Horizontal),
        ship(5, 4, 1, Horizontal),
        ship(7, 4, 1, Horizontal),
        ship(9, 4, 1, Horizontal),
    ]
}

#[test]
fn commands_secret_commands_and_world_events_round_trip() {
    for command in [Command::Fire(Cell::new(9, 3).unwrap()), Command::GiveUp] {
        let wire = binary(&SeaBattle::command_to_wire(&command));
        assert_eq!(SeaBattle::command_from_wire(&wire), Ok(command));
    }
    let placement = SecretCommand::PlaceFleet(classic_ships());
    let wire = binary(&SeaBattle::secret_command_to_wire(&placement));
    assert_eq!(SeaBattle::secret_command_from_wire(&wire), Ok(placement));
    for event in [
        WorldEvent::FleetPlaced(Slot::new(1)),
        WorldEvent::FirstShooterChosen(Slot::new(0)),
        WorldEvent::ShotResolved(ShotOutcome::Miss),
        WorldEvent::ShotResolved(ShotOutcome::Hit),
        WorldEvent::ShotResolved(ShotOutcome::Sunk(ship(2, 6, 3, Orientation::Vertical))),
        WorldEvent::FleetRevealed {
            slot: Slot::new(1),
            ships: classic_ships(),
        },
    ] {
        let wire = binary(&SeaBattle::world_event_to_wire(&event));
        assert_eq!(SeaBattle::world_event_from_wire(&wire), Ok(event));
    }
    for kind in [BotKind::Easy, BotKind::Medium, BotKind::Hard] {
        let wire = binary(&SeaBattle::bot_kind_to_wire(kind));
        assert_eq!(SeaBattle::bot_kind_from_wire(&wire), Ok(kind));
    }
}

#[test]
fn cells_and_ships_off_the_board_are_refused_on_the_wire() {
    let fire = |x, y| proto::GameCommand {
        game: Some(proto::game_command::Game::SeaBattle(
            proto::SeaBattleCommand {
                command: Some(proto::sea_battle_command::Command::Fire(
                    proto::SeaBattleFire {
                        cell: Some(proto::SeaBattleCell { x, y }),
                    },
                )),
            },
        )),
    };
    assert_eq!(
        SeaBattle::command_from_wire(&fire(10, 0)),
        Err(WireError::OutOfRange("cell"))
    );
    assert_eq!(
        SeaBattle::command_from_wire(&fire(u32::MAX, 0)),
        Err(WireError::OutOfRange("x"))
    );
    let placement = |ships: Vec<proto::SeaBattleShip>| proto::GameSecretCommand {
        game: Some(proto::game_secret_command::Game::SeaBattle(
            proto::SeaBattleSecretCommand {
                command: Some(proto::sea_battle_secret_command::Command::PlaceFleet(
                    proto::SeaBattlePlaceFleet { ships },
                )),
            },
        )),
    };
    let overhanging = proto::SeaBattleShip {
        bow: Some(proto::SeaBattleCell { x: 8, y: 0 }),
        length: 4,
        orientation: proto::SeaBattleOrientation::Horizontal.into(),
    };
    assert_eq!(
        SeaBattle::secret_command_from_wire(&placement(vec![overhanging])),
        Err(WireError::OutOfRange("ship"))
    );
    let single = proto::SeaBattleShip {
        bow: Some(proto::SeaBattleCell { x: 0, y: 0 }),
        length: 1,
        orientation: proto::SeaBattleOrientation::Horizontal.into(),
    };
    assert_eq!(
        SeaBattle::secret_command_from_wire(&placement(vec![single; 11])),
        Err(WireError::OutOfRange("ships"))
    );
}

/// Games without a secret refuse a private command as another game's.
#[test]
fn another_game_takes_no_private_command() {
    let placement = SeaBattle::secret_command_to_wire(&SecretCommand::PlaceFleet(classic_ships()));
    assert_eq!(
        <mg_checkers::Checkers as WireGame>::secret_command_from_wire(&placement),
        Err(WireError::WrongGame)
    );
}

#[test]
fn the_seat_secret_carries_the_own_fleet() {
    let mut authority =
        Authority::<SeaBattle>::new(&Settings, &[PlayerSpec::HUMAN; 2], 3, Vec::new()).unwrap();
    authority
        .submit_secret(Slot::new(0), &SecretCommand::PlaceFleet(classic_ships()))
        .unwrap();
    let fleet = authority.seat_secret(Slot::new(0)).unwrap();
    let Some(proto::game_seat_secret::Game::SeaBattle(secret)) =
        binary(&SeaBattle::seat_secret_to_wire(&fleet)).game
    else {
        panic!("not a sea battle secret");
    };
    assert_eq!(secret.ships.len(), 10);
}

/// Authority frames go through the JSON mapping to a client, which ends with the same hash and
/// never sees a fleet before the end.
#[cfg(feature = "json")]
#[test]
fn authority_frames_replay_through_json() {
    let players = [PlayerSpec::HUMAN; 2];
    let mut authority = Authority::<SeaBattle>::new(&Settings, &players, 5, Vec::new()).unwrap();
    let mut client = Match::<SeaBattle>::new(&Settings, &players).unwrap();
    let mut deliver = |frame: &mg_core::GameFrame<SeaBattle>| {
        let json = serde_json::to_string(&crate::frame_to_wire::<SeaBattle>(frame)).unwrap();
        let wire: proto::TickFrame = serde_json::from_str(&json).unwrap();
        client
            .apply_frame(&crate::frame_from_wire::<SeaBattle>(&wire).unwrap())
            .unwrap();
        // Only the frame that ends the game may carry the fleets.
        assert_eq!(
            json.contains("fleetRevealed"),
            client.game().status().is_finished(),
            "{json}"
        );
    };
    deliver(&authority.seal_frame());
    for slot in [Slot::new(0), Slot::new(1)] {
        authority
            .submit_secret(slot, &SecretCommand::PlaceFleet(classic_ships()))
            .unwrap();
        deliver(&authority.seal_frame());
    }
    let targets: Vec<Cell> = classic_ships()
        .iter()
        .flat_map(|ship| ship.cells())
        .collect();
    for target in targets {
        let mg_core::Awaiting::Turn(shooter) = authority.game().awaiting() else {
            panic!("nobody to fire");
        };
        authority.submit(shooter, Command::Fire(target)).unwrap();
        deliver(&authority.seal_frame());
    }
    assert!(client.game().status().is_finished());
    assert_eq!(client.game().state_hash(), authority.game().state_hash());
}
