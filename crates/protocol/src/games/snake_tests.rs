use mg_core::{
    Authority, BotSeat, CommandError, CoreRng, Game, GameFrame, GameSettings, HasBots, Match,
    OutcomeKind, PlayerSpec, Slot, Status, Viewer,
};
use mg_snake::{
    BotKind, Command, DeadSnakeBehavior, Direction, Point, RawSettings, Settings, Snake,
    WallCollisionMode, WorldEvent,
};
use prost::Message;

use crate::proto;
use crate::{WireBots, WireError, WireGame, frame_from_wire, frame_to_wire, slot_to_wire};

fn snake_settings_wire(settings: proto::SnakeSettings) -> proto::GameSettings {
    proto::GameSettings {
        game: Some(proto::game_settings::Game::Snake(settings)),
    }
}

fn valid_settings_wire() -> proto::SnakeSettings {
    match Snake::settings_to_wire(&RawSettings::default()).game {
        Some(proto::game_settings::Game::Snake(settings)) => settings,
        other => panic!("unexpected {other:?}"),
    }
}

fn spawn_food_wire(cell: Option<proto::SnakePoint>) -> proto::GameWorldEvent {
    proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::Snake(
            proto::SnakeWorldEvent {
                event: Some(proto::snake_world_event::Event::SpawnFood(
                    proto::SnakeSpawnFood { cell },
                )),
            },
        )),
    }
}

fn turn_wire(direction: i32) -> proto::GameCommand {
    proto::GameCommand {
        game: Some(proto::game_command::Game::Snake(proto::SnakeCommand {
            command: Some(proto::snake_command::Command::Turn(proto::SnakeTurn {
                direction,
            })),
        })),
    }
}

#[test]
fn settings_round_trip() {
    let variants = [
        RawSettings::default(),
        RawSettings {
            field_width: 100,
            field_height: 10,
            wall_collision_mode: WallCollisionMode::Death,
            dead_snake_behavior: DeadSnakeBehavior::StayOnField,
            max_food_count: 50,
            food_spawn_permille: 1,
            tick_interval_ms: 5000,
        },
    ];
    for settings in variants {
        let wire = Snake::settings_to_wire(&settings);
        assert_eq!(Snake::settings_from_wire(&wire), Ok(settings));
    }
}

#[test]
fn commands_round_trip() {
    for direction in Direction::ALL {
        let command = Command::Turn(direction);
        let wire = Snake::command_to_wire(&command);
        assert_eq!(Snake::command_from_wire(&wire), Ok(command));
    }
}

#[test]
fn world_events_round_trip() {
    for cell in [
        Point::new(0, 0),
        Point::new(99, 42),
        Point::new(u16::MAX, 7),
    ] {
        let event = WorldEvent::SpawnFood(cell);
        let wire = Snake::world_event_to_wire(&event);
        assert_eq!(Snake::world_event_from_wire(&wire), Ok(event));
    }
}

#[test]
fn bot_kinds_round_trip() {
    for &kind in Snake::BOT_KINDS {
        let wire = Snake::bot_kind_to_wire(kind);
        assert_eq!(Snake::bot_kind_from_wire(&wire), Ok(kind));
    }
}

#[test]
fn view_carries_seats_food_and_end() {
    let players = [PlayerSpec::HUMAN, PlayerSpec::BOT];
    let mut game = Snake::new(&Settings::default(), &players).unwrap();
    game.apply_world(&WorldEvent::SpawnFood(Point::new(0, 0)))
        .unwrap();
    game.apply_command(Slot::new(0), &Command::Turn(Direction::Left))
        .unwrap();
    game.drop_player(Slot::new(1));
    let Some(proto::game_view::Game::Snake(view)) =
        Snake::view_to_wire(&game.view(Viewer::Player(Slot::new(0)))).game
    else {
        panic!("not a snake view");
    };
    assert_eq!((view.field_width, view.field_height), (15, 15));
    assert_eq!(view.you, Some(0));
    assert_eq!(view.food, [proto::SnakePoint { x: 0, y: 0 }]);
    assert_eq!(view.snakes.len(), 2);
    let human = &view.snakes[0];
    assert!(human.alive && !human.is_bot);
    assert_eq!(human.body.len(), 3);
    assert_eq!(human.queued_turns, [i32::from(proto::SnakeDirection::Left)]);
    let bot = &view.snakes[1];
    assert!(!bot.alive && bot.is_bot);
    assert_eq!(
        bot.death_reason,
        i32::from(proto::SnakeDeathReason::PlayerDisconnected)
    );
    let end = view.end.expect("match ended");
    assert_eq!(end.winner_slots, [0]);
    assert_eq!(
        end.reason,
        i32::from(proto::SnakeDeathReason::PlayerDisconnected)
    );
}

#[test]
fn view_end_lists_every_surviving_bot_as_a_winner() {
    let players = [PlayerSpec::HUMAN, PlayerSpec::BOT, PlayerSpec::BOT];
    let mut game = Snake::new(&Settings::default(), &players).unwrap();
    game.drop_player(Slot::new(0));
    let Status::Finished(outcome) = game.status() else {
        panic!("the match must end once no human is left");
    };
    assert_eq!(outcome.kind, OutcomeKind::Winner);
    let Some(proto::game_view::Game::Snake(view)) =
        Snake::view_to_wire(&game.view(Viewer::Player(Slot::new(0)))).game
    else {
        panic!("not a snake view");
    };
    let end = view.end.expect("match ended");
    assert_eq!(end.winner_slots, [1, 2]);
    let outcome_winners: Vec<u32> = outcome.winners.iter().copied().map(slot_to_wire).collect();
    assert_eq!(end.winner_slots, outcome_winners);
}

#[test]
fn malformed_settings_are_rejected() {
    assert_eq!(
        Snake::settings_from_wire(&proto::GameSettings { game: None }),
        Err(WireError::Missing("game"))
    );
    let other_game = proto::GameSettings {
        game: Some(proto::game_settings::Game::Tictactoe(
            proto::TicTacToeSettings::default(),
        )),
    };
    assert_eq!(
        Snake::settings_from_wire(&other_game),
        Err(WireError::WrongGame)
    );
    let unspecified_walls = proto::SnakeSettings {
        wall_collision_mode: 0,
        ..valid_settings_wire()
    };
    assert_eq!(
        Snake::settings_from_wire(&snake_settings_wire(unspecified_walls)),
        Err(WireError::Missing("wall_collision_mode"))
    );
    let unknown_behavior = proto::SnakeSettings {
        dead_snake_behavior: 99,
        ..valid_settings_wire()
    };
    assert_eq!(
        Snake::settings_from_wire(&snake_settings_wire(unknown_behavior)),
        Err(WireError::UnknownEnumValue("dead_snake_behavior"))
    );
}

#[test]
fn out_of_range_sizes_pass_the_wire_and_are_rejected_by_the_game() {
    let huge = proto::SnakeSettings {
        field_width: u32::MAX,
        ..valid_settings_wire()
    };
    let settings = Snake::settings_from_wire(&snake_settings_wire(huge)).unwrap();
    assert_eq!(settings.field_width, u32::MAX);
    assert_eq!(
        Settings::parse(settings).map_err(|error| error.field),
        Err("field_width")
    );
}

#[test]
fn malformed_commands_and_events_are_rejected() {
    assert_eq!(
        Snake::command_from_wire(&turn_wire(0)),
        Err(WireError::Missing("direction"))
    );
    assert_eq!(
        Snake::command_from_wire(&turn_wire(17)),
        Err(WireError::UnknownEnumValue("direction"))
    );
    let empty_command = proto::GameCommand {
        game: Some(proto::game_command::Game::Snake(proto::SnakeCommand {
            command: None,
        })),
    };
    assert_eq!(
        Snake::command_from_wire(&empty_command),
        Err(WireError::Missing("command"))
    );
    let other_game = proto::GameCommand {
        game: Some(proto::game_command::Game::Tictactoe(
            proto::TicTacToeCommand::default(),
        )),
    };
    assert_eq!(
        Snake::command_from_wire(&other_game),
        Err(WireError::WrongGame)
    );

    assert_eq!(
        Snake::world_event_from_wire(&spawn_food_wire(None)),
        Err(WireError::Missing("cell"))
    );
    let too_far = proto::SnakePoint {
        x: 3,
        y: u32::from(u16::MAX) + 1,
    };
    assert_eq!(
        Snake::world_event_from_wire(&spawn_food_wire(Some(too_far))),
        Err(WireError::IntegerOutOfRange {
            field: "y",
            source: u16::try_from(u32::MAX).unwrap_err(),
        })
    );
    let empty_event = proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::Snake(
            proto::SnakeWorldEvent { event: None },
        )),
    };
    assert_eq!(
        Snake::world_event_from_wire(&empty_event),
        Err(WireError::Missing("event"))
    );
}

#[test]
fn malformed_bot_kinds_are_rejected() {
    let snake_kind = |value: i32| proto::BotKind {
        game: Some(proto::bot_kind::Game::Snake(value)),
    };
    assert_eq!(
        Snake::bot_kind_from_wire(&snake_kind(0)),
        Err(WireError::Missing("bot_kind"))
    );
    assert_eq!(
        Snake::bot_kind_from_wire(&snake_kind(3)),
        Err(WireError::UnknownEnumValue("bot_kind"))
    );
    let other_game = proto::BotKind {
        game: Some(proto::bot_kind::Game::Tictactoe(1)),
    };
    assert_eq!(
        Snake::bot_kind_from_wire(&other_game),
        Err(WireError::WrongGame)
    );
}

/// Runs an authority with humans and bots, ships every sealed frame through `transport` and
/// replays it on a `Match`; returns what crossed the wire.
fn replay_over_the_wire(transport: impl Fn(&proto::TickFrame) -> proto::TickFrame) -> TrafficSeen {
    let settings = Settings::parse(RawSettings {
        field_width: 40,
        field_height: 40,
        max_food_count: 4,
        food_spawn_permille: 400,
        ..RawSettings::default()
    })
    .unwrap();
    let players = [
        PlayerSpec::HUMAN,
        PlayerSpec::BOT,
        PlayerSpec::HUMAN,
        PlayerSpec::BOT,
    ];
    let bots = [
        (Slot::new(1), BotKind::Efficient),
        (Slot::new(3), BotKind::Random),
    ]
    .into_iter()
    .map(|(slot, kind)| BotSeat {
        slot,
        bot: Snake::make_bot(kind),
        budget_nodes: Snake::default_bot_budget(kind),
    })
    .collect();
    let mut authority = Authority::<Snake>::new(&settings, &players, 2024, bots).unwrap();
    let mut mirror = Match::<Snake>::new(&settings, &players).unwrap();
    let mut fuzz = CoreRng::new(7);
    let mut traffic = TrafficSeen::default();
    for tick in 0..400 {
        if authority.game().status().is_finished() {
            break;
        }
        for slot in [Slot::new(0), Slot::new(2)] {
            if fuzz.chance(1, 12) {
                let direction = *fuzz.pick(&Direction::ALL).unwrap();
                // Rejected turns (reversal, full buffer, dead snake) never reach a frame.
                if let Err(error) = authority.submit(slot, Command::Turn(direction)) {
                    assert_ne!(error, CommandError::GameOver);
                }
            }
        }
        if tick == 150 {
            authority.drop_player(Slot::new(3));
        }
        authority.run_bots();
        let frame: GameFrame<Snake> = authority.seal_frame();
        let received =
            frame_from_wire::<Snake>(&transport(&frame_to_wire::<Snake>(&frame))).unwrap();
        assert_eq!(received, frame);
        traffic.count(&received);
        mirror.apply_frame(&received).unwrap();
    }
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
    traffic
}

#[derive(Debug, Default)]
struct TrafficSeen {
    frames: usize,
    commands: usize,
    world_events: usize,
    drops: usize,
}

impl TrafficSeen {
    fn count(&mut self, frame: &GameFrame<Snake>) {
        self.frames += 1;
        for input in &frame.inputs {
            match input {
                mg_core::Input::Command { .. } => self.commands += 1,
                mg_core::Input::World(_) => self.world_events += 1,
                mg_core::Input::Dropped { .. } => self.drops += 1,
            }
        }
    }

    fn assert_exercised_everything(&self) {
        assert!(self.frames >= 200, "{self:?}");
        assert!(self.commands > 0, "{self:?}");
        assert!(self.world_events > 0, "{self:?}");
        assert_eq!(self.drops, 1, "{self:?}");
    }
}

#[test]
fn authority_frames_replay_identically_after_binary_encoding() {
    let traffic = replay_over_the_wire(|frame| {
        proto::TickFrame::decode(frame.encode_to_vec().as_slice()).unwrap()
    });
    traffic.assert_exercised_everything();
}

#[cfg(feature = "json")]
#[test]
fn authority_frames_replay_identically_after_json_encoding() {
    let traffic = replay_over_the_wire(|frame| {
        let json = serde_json::to_string(frame).unwrap();
        serde_json::from_str(&json).unwrap()
    });
    traffic.assert_exercised_everything();
}
