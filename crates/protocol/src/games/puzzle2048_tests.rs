use mg_core::{CoreRng, Game, PlayerSpec, Viewer};
use mg_puzzle2048::{
    Cell, Command, Direction, Puzzle2048, Settings, SpawnValue, TileOrigin, WorldEvent,
};
use prost::Message;

use crate::proto;
use crate::wire_error::WireError;
use crate::wire_game::WireGame;

/// Sends a message through the binary encoding and, with feature `json`, through protobuf JSON.
fn transport<M>(message: &M) -> M
where
    M: Message + Default + PartialEq + std::fmt::Debug + JsonTransport,
{
    let decoded = M::decode(message.encode_to_vec().as_slice()).unwrap();
    assert_eq!(&decoded, message);
    decoded.through_json()
}

#[cfg(feature = "json")]
trait JsonTransport: serde::Serialize + serde::de::DeserializeOwned {
    fn through_json(&self) -> Self {
        serde_json::from_str(&serde_json::to_string(self).unwrap()).unwrap()
    }
}

#[cfg(feature = "json")]
impl<M: serde::Serialize + serde::de::DeserializeOwned> JsonTransport for M {}

#[cfg(not(feature = "json"))]
trait JsonTransport: Clone {
    fn through_json(&self) -> Self {
        self.clone()
    }
}

#[cfg(not(feature = "json"))]
impl<M: Clone> JsonTransport for M {}

#[test]
fn settings_round_trip_unvalidated() {
    let samples = [
        Settings::default(),
        Settings {
            width: 3,
            height: 7,
            target: 64,
        },
        // Rule validation belongs to the game: the wire keeps even invalid values intact.
        Settings {
            width: 0,
            height: u32::MAX,
            target: 3,
        },
    ];
    for settings in samples {
        let wire = transport(&Puzzle2048::settings_to_wire(&settings));
        assert_eq!(Puzzle2048::settings_from_wire(&wire), Ok(settings));
    }
}

#[test]
fn commands_round_trip() {
    for direction in Direction::ALL {
        let command = Command::Move(direction);
        let wire = transport(&Puzzle2048::command_to_wire(&command));
        assert_eq!(Puzzle2048::command_from_wire(&wire), Ok(command));
    }
}

#[test]
fn world_events_round_trip() {
    for (row, col) in [(0, 0), (9, 9), (3, 0), (u8::MAX, u8::MAX)] {
        for value in [SpawnValue::Two, SpawnValue::Four] {
            let event = WorldEvent::TileSpawned {
                cell: Cell::new(row, col),
                value,
            };
            let wire = transport(&Puzzle2048::world_event_to_wire(&event));
            assert_eq!(Puzzle2048::world_event_from_wire(&wire), Ok(event));
        }
    }
}

fn command_with_direction(raw: i32) -> proto::GameCommand {
    proto::GameCommand {
        game: Some(proto::game_command::Game::Puzzle2048(
            proto::Puzzle2048Command {
                command: Some(proto::puzzle2048_command::Command::Move(raw)),
            },
        )),
    }
}

#[test]
fn malformed_commands_are_rejected() {
    let cases = [
        (
            proto::GameCommand { game: None },
            WireError::Missing("game"),
        ),
        (
            proto::GameCommand {
                game: Some(proto::game_command::Game::Snake(
                    proto::SnakeCommand::default(),
                )),
            },
            WireError::WrongGame,
        ),
        (
            proto::GameCommand {
                game: Some(proto::game_command::Game::Puzzle2048(
                    proto::Puzzle2048Command { command: None },
                )),
            },
            WireError::Missing("command"),
        ),
        (command_with_direction(0), WireError::Missing("move")),
        (
            command_with_direction(5),
            WireError::UnknownEnumValue("move"),
        ),
        (
            command_with_direction(-1),
            WireError::UnknownEnumValue("move"),
        ),
    ];
    for (wire, expected) in cases {
        assert_eq!(
            Puzzle2048::command_from_wire(&wire),
            Err(expected),
            "{wire:?}"
        );
    }
}

fn spawn_event(cell: Option<proto::Puzzle2048Cell>, value: i32) -> proto::GameWorldEvent {
    proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::Puzzle2048(
            proto::Puzzle2048WorldEvent {
                event: Some(proto::puzzle2048_world_event::Event::TileSpawned(
                    proto::Puzzle2048TileSpawned { cell, value },
                )),
            },
        )),
    }
}

fn wire_cell(row: u32, col: u32) -> Option<proto::Puzzle2048Cell> {
    Some(proto::Puzzle2048Cell { row, col })
}

#[test]
fn malformed_world_events_are_rejected() {
    let two = i32::from(proto::Puzzle2048SpawnValue::Two);
    let cases = [
        (
            proto::GameWorldEvent { game: None },
            WireError::Missing("game"),
        ),
        (
            proto::GameWorldEvent {
                game: Some(proto::game_world_event::Game::Tictactoe(
                    proto::TicTacToeWorldEvent::default(),
                )),
            },
            WireError::WrongGame,
        ),
        (
            proto::GameWorldEvent {
                game: Some(proto::game_world_event::Game::Puzzle2048(
                    proto::Puzzle2048WorldEvent { event: None },
                )),
            },
            WireError::Missing("event"),
        ),
        (spawn_event(None, two), WireError::Missing("cell")),
        (
            spawn_event(wire_cell(256, 0), two),
            WireError::IntegerOutOfRange {
                field: "cell.row",
                source: u8::try_from(256_u32).unwrap_err(),
            },
        ),
        (
            spawn_event(wire_cell(0, u32::MAX), two),
            WireError::IntegerOutOfRange {
                field: "cell.col",
                source: u8::try_from(u32::MAX).unwrap_err(),
            },
        ),
        (spawn_event(wire_cell(0, 0), 0), WireError::Missing("value")),
        (
            spawn_event(wire_cell(0, 0), 3),
            WireError::UnknownEnumValue("value"),
        ),
    ];
    for (wire, expected) in cases {
        assert_eq!(
            Puzzle2048::world_event_from_wire(&wire),
            Err(expected),
            "{wire:?}"
        );
    }
}

#[test]
fn malformed_settings_are_rejected() {
    assert_eq!(
        Puzzle2048::settings_from_wire(&proto::GameSettings { game: None }),
        Err(WireError::Missing("game"))
    );
    let snake = proto::GameSettings {
        game: Some(proto::game_settings::Game::Snake(
            proto::SnakeSettings::default(),
        )),
    };
    assert_eq!(
        Puzzle2048::settings_from_wire(&snake),
        Err(WireError::WrongGame)
    );
}

fn legal_direction(game: &Puzzle2048, rng: &mut CoreRng) -> Option<Direction> {
    let legal: Vec<Direction> = Direction::ALL
        .into_iter()
        .filter(|&direction| game.can_move(direction))
        .collect();
    rng.pick(&legal).copied()
}

fn wire_direction(direction: Direction) -> proto::Puzzle2048Direction {
    match direction {
        Direction::Up => proto::Puzzle2048Direction::Up,
        Direction::Down => proto::Puzzle2048Direction::Down,
        Direction::Left => proto::Puzzle2048Direction::Left,
        Direction::Right => proto::Puzzle2048Direction::Right,
    }
}

fn wire_view(game: &Puzzle2048) -> proto::Puzzle2048View {
    match Puzzle2048::view_to_wire(&game.view(Viewer::Spectator)).game {
        Some(proto::game_view::Game::Puzzle2048(view)) => view,
        other => panic!("not a 2048 view: {other:?}"),
    }
}

#[test]
fn view_carries_tiles_and_animation_origins() {
    let mut authority = mg_core::Authority::<Puzzle2048>::new(
        &Settings::default(),
        &[PlayerSpec::HUMAN],
        7,
        vec![],
    )
    .unwrap();
    authority.seal_frame();
    let mut rng = CoreRng::new(7);
    let mut saw_merge = false;
    while !saw_merge {
        let direction = legal_direction(authority.game(), &mut rng).unwrap();
        authority.submit(PLAYER, Command::Move(direction)).unwrap();
        authority.seal_frame();
        let domain = authority.game().view(Viewer::Spectator);
        let wire = wire_view(authority.game());
        assert_eq!(wire.tiles.len(), domain.tiles.len());
        assert_eq!(u64::from(wire.score), domain.score.get());
        assert_eq!(wire.moves, domain.moves);
        assert_eq!(wire.last_move(), wire_direction(direction));
        let movable: Vec<i32> = domain
            .movable_directions
            .iter()
            .map(|&direction| wire_direction(direction).into())
            .collect();
        assert!(!movable.is_empty());
        assert_eq!(wire.movable_directions, movable);
        assert_eq!(wire.phase(), proto::Puzzle2048Phase::Playing);
        for (wire_tile, tile) in wire.tiles.iter().zip(&domain.tiles) {
            assert_eq!(wire_tile.id, tile.id.get());
            assert_eq!(wire_tile.value, tile.value.get());
            assert_eq!(
                wire_tile.cell,
                Some(proto::Puzzle2048Cell {
                    row: u32::from(tile.cell.row),
                    col: u32::from(tile.cell.col),
                })
            );
            match (&wire_tile.origin, tile.origin) {
                (Some(proto::puzzle2048_tile::Origin::Slid(slid)), TileOrigin::Slid { from }) => {
                    assert_eq!(slid.from, wire_cell(from.row.into(), from.col.into()));
                }
                (
                    Some(proto::puzzle2048_tile::Origin::Merged(merged)),
                    TileOrigin::Merged { sources },
                ) => {
                    saw_merge = true;
                    let ids: Vec<u32> = merged.sources.iter().map(|source| source.id).collect();
                    assert_eq!(ids, [sources[0].id.get(), sources[1].id.get()]);
                }
                (Some(proto::puzzle2048_tile::Origin::Spawned(_)), TileOrigin::Spawned) => {}
                (wire_origin, origin) => panic!("{wire_origin:?} does not match {origin:?}"),
            }
        }
    }
}

const PLAYER: mg_core::Slot = mg_core::Slot::new(0);

#[cfg(feature = "json")]
fn deliver(frame: &mg_core::GameFrame<Puzzle2048>, client: &mut mg_core::Match<Puzzle2048>) {
    let json = serde_json::to_string(&crate::frame_to_wire::<Puzzle2048>(frame)).unwrap();
    let wire: proto::TickFrame = serde_json::from_str(&json).unwrap();
    let decoded = crate::frame_from_wire::<Puzzle2048>(&wire).unwrap();
    assert_eq!(&decoded, frame);
    client.apply_frame(&decoded).unwrap();
}

/// The authority's frames, sent as protobuf JSON, replay to the same state on a client.
#[cfg(feature = "json")]
#[test]
fn authority_frames_replay_through_json() {
    for (seed, settings) in [
        (
            1,
            Settings {
                width: 3,
                height: 3,
                target: 64,
            },
        ),
        (2, Settings::default()),
    ] {
        let players = [PlayerSpec::HUMAN];
        let mut authority =
            mg_core::Authority::<Puzzle2048>::new(&settings, &players, seed, vec![]).unwrap();
        let mut client = mg_core::Match::<Puzzle2048>::new(&settings, &players).unwrap();
        let mut rng = CoreRng::new(seed);

        deliver(&authority.seal_frame(), &mut client);
        for _ in 0..400 {
            let Some(direction) = legal_direction(authority.game(), &mut rng) else {
                break;
            };
            authority.submit(PLAYER, Command::Move(direction)).unwrap();
            deliver(&authority.seal_frame(), &mut client);
        }
        if !authority.game().status().is_finished() {
            authority.drop_player(PLAYER);
            deliver(&authority.seal_frame(), &mut client);
        }

        assert!(client.game().status().is_finished());
        assert_eq!(client.game().state_hash(), authority.game().state_hash());
        assert_eq!(wire_view(client.game()), wire_view(authority.game()));
    }
}
