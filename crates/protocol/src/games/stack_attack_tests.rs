use mg_core::{Authority, CoreRng, Game, Match, PlayerSpec, Slot, Viewer};
use mg_stack_attack::{Command, Direction, PatternId, Settings, Side, StackAttack, WorldEvent};

use crate::proto;
use crate::proto::{game_command, game_settings, game_world_event};
use crate::proto::{stack_attack_command, stack_attack_world_event};
use crate::wire_error::WireError;
use crate::wire_game::WireGame;
use crate::{frame_from_wire, frame_to_wire};

fn crane_spawn(target_x: i32, from_side: i32, pattern_id: u32) -> proto::GameWorldEvent {
    proto::GameWorldEvent {
        game: Some(game_world_event::Game::StackAttack(
            proto::StackAttackWorldEvent {
                event: Some(stack_attack_world_event::Event::CraneSpawn(
                    proto::StackAttackCraneSpawn {
                        target_x,
                        from_side,
                        pattern_id,
                    },
                )),
            },
        )),
    }
}

fn move_command(direction: i32) -> proto::GameCommand {
    proto::GameCommand {
        game: Some(game_command::Game::StackAttack(proto::StackAttackCommand {
            command: Some(stack_attack_command::Command::Move(
                proto::StackAttackMove { direction },
            )),
        })),
    }
}

#[test]
fn settings_round_trip() {
    let settings = Settings {
        field_width: 20,
        field_height: 12,
    };
    let wire = StackAttack::settings_to_wire(&settings);
    assert_eq!(StackAttack::settings_from_wire(&wire), Ok(settings));
}

#[test]
fn commands_round_trip() {
    for command in [
        Command::Move(Direction::Left),
        Command::Move(Direction::Right),
        Command::Jump,
    ] {
        let wire = StackAttack::command_to_wire(&command);
        assert_eq!(StackAttack::command_from_wire(&wire), Ok(command));
    }
}

#[test]
fn world_events_round_trip() {
    for (target_x, from) in [(0, Side::Left), (14, Side::Right)] {
        for pattern in 0..PatternId::COUNT {
            let event = WorldEvent::CraneSpawned {
                target_x,
                from,
                pattern: PatternId::new(pattern).unwrap(),
            };
            let wire = StackAttack::world_event_to_wire(&event);
            assert_eq!(StackAttack::world_event_from_wire(&wire), Ok(event));
        }
    }
}

#[test]
fn other_games_messages_are_rejected() {
    let settings = proto::GameSettings {
        game: Some(game_settings::Game::Snake(Default::default())),
    };
    assert_eq!(
        StackAttack::settings_from_wire(&settings),
        Err(WireError::WrongGame)
    );
    let command = proto::GameCommand {
        game: Some(game_command::Game::Snake(Default::default())),
    };
    assert_eq!(
        StackAttack::command_from_wire(&command),
        Err(WireError::WrongGame)
    );
    let event = proto::GameWorldEvent {
        game: Some(game_world_event::Game::Snake(Default::default())),
    };
    assert_eq!(
        StackAttack::world_event_from_wire(&event),
        Err(WireError::WrongGame)
    );
}

#[test]
fn malformed_messages_are_rejected() {
    assert_eq!(
        StackAttack::settings_from_wire(&proto::GameSettings::default()),
        Err(WireError::Missing("game"))
    );
    let empty_command = proto::GameCommand {
        game: Some(game_command::Game::StackAttack(Default::default())),
    };
    assert_eq!(
        StackAttack::command_from_wire(&empty_command),
        Err(WireError::Missing("command"))
    );
    assert_eq!(
        StackAttack::command_from_wire(&move_command(0)),
        Err(WireError::Missing("direction"))
    );
    assert_eq!(
        StackAttack::command_from_wire(&move_command(7)),
        Err(WireError::UnknownEnumValue("direction"))
    );

    let empty_event = proto::GameWorldEvent {
        game: Some(game_world_event::Game::StackAttack(Default::default())),
    };
    assert_eq!(
        StackAttack::world_event_from_wire(&empty_event),
        Err(WireError::Missing("event"))
    );
    let left = proto::StackAttackSide::Left as i32;
    assert_eq!(
        StackAttack::world_event_from_wire(&crane_spawn(3, 0, 1)),
        Err(WireError::Missing("from_side"))
    );
    assert_eq!(
        StackAttack::world_event_from_wire(&crane_spawn(3, 9, 1)),
        Err(WireError::UnknownEnumValue("from_side"))
    );
    for pattern_id in [u32::from(PatternId::COUNT), 256, u32::MAX] {
        assert_eq!(
            StackAttack::world_event_from_wire(&crane_spawn(3, left, pattern_id)),
            Err(WireError::OutOfRange("pattern_id"))
        );
    }
}

#[test]
fn view_carries_the_render_model() {
    let players = [PlayerSpec::HUMAN, PlayerSpec::HUMAN];
    let mut game = StackAttack::new(&Settings::default(), &players).unwrap();
    game.drop_player(Slot::new(1));
    let proto::GameView {
        game: Some(proto::game_view::Game::StackAttack(view)),
    } = StackAttack::view_to_wire(&game.view(Viewer::Player(Slot::new(0))))
    else {
        panic!("wrong view envelope");
    };
    assert_eq!(view.you, Some(0));
    assert_eq!((view.field_width, view.field_height), (15, 10));
    assert_eq!(view.tick_ms, 200);
    assert_eq!(view.workers.len(), 2);
    assert_eq!(
        view.workers[1].status,
        proto::StackAttackWorkerStatus::Left as i32
    );
    assert_eq!(view.end_reason, None);

    game.drop_player(Slot::new(0));
    let proto::GameView {
        game: Some(proto::game_view::Game::StackAttack(view)),
    } = StackAttack::view_to_wire(&game.view(Viewer::Spectator))
    else {
        panic!("wrong view envelope");
    };
    assert_eq!(view.you, None);
    assert_eq!(
        view.end_reason,
        Some(proto::StackAttackEndReason::AllWorkersLeft as i32)
    );
    assert!(view.events.iter().any(|event| matches!(
        event.event,
        Some(proto::stack_attack_event::Event::WorkerLeft(
            proto::StackAttackWorkerLeftEvent { slot: 0 }
        ))
    )));
}

#[cfg(feature = "json")]
#[test]
fn frames_survive_json_and_replay_to_the_same_state() {
    let settings = Settings::default();
    let players = [PlayerSpec::HUMAN; 3];
    let mut authority = Authority::<StackAttack>::new(&settings, &players, 11, vec![]).unwrap();
    let mut mirror = Match::<StackAttack>::new(&settings, &players).unwrap();
    let mut fuzz = CoreRng::new(5);
    let commands = [
        Command::Move(Direction::Left),
        Command::Move(Direction::Right),
        Command::Jump,
    ];
    let mut world_events = 0;
    for tick in 0..400 {
        for slot in Slot::all(players.len()) {
            if fuzz.chance(1, 2) {
                // Busy / NotPlaying rejections are expected and never reach a frame.
                authority.submit(slot, *fuzz.pick(&commands).unwrap()).ok();
            }
        }
        if tick == 250 {
            authority.drop_player(Slot::new(2));
        }
        let frame = authority.seal_frame();
        world_events += frame
            .inputs
            .iter()
            .filter(|input| matches!(input, mg_core::Input::World(_)))
            .count();
        let json = serde_json::to_string(&frame_to_wire::<StackAttack>(&frame)).unwrap();
        let wire: proto::TickFrame = serde_json::from_str(&json).unwrap();
        let decoded = frame_from_wire::<StackAttack>(&wire).unwrap();
        assert_eq!(decoded, frame);
        mirror.apply_frame(&decoded).unwrap();
        if authority.game().status().is_finished() {
            break;
        }
    }
    let frames = mirror.next_tick().get();
    assert!(frames >= 200, "match ended after {frames} frames");
    assert!(
        world_events > 0,
        "no crane spawned, the test exercised nothing"
    );
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
}
