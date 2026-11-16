use mg_core::{Authority, Awaiting, BotSeat, Game, HasBots, Match, PlayerSpec, Slot, Viewer};
use mg_tictactoe::{
    BotKind, Command, FirstPlayerMode, GameEnd, Mark, Point, Settings, TicTacToe, WinningLine,
    WorldEvent,
};
use prost::Message;

use crate::proto::{self, TicTacToeEndReason, TicTacToeMark};
use crate::{WireBots, WireError, WireGame, frame_from_wire, frame_to_wire};

fn settings(first_player: FirstPlayerMode) -> Settings {
    Settings {
        field_width: 9,
        field_height: 7,
        win_count: 4,
        first_player,
    }
}

fn wire_settings(first_player: i32) -> proto::GameSettings {
    proto::GameSettings {
        game: Some(proto::game_settings::Game::Tictactoe(
            proto::TicTacToeSettings {
                field_width: 15,
                field_height: 15,
                win_count: 5,
                first_player,
            },
        )),
    }
}

fn wire_first_player(x_slot: u32) -> proto::GameWorldEvent {
    proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::Tictactoe(
            proto::TicTacToeWorldEvent {
                event: Some(proto::tic_tac_toe_world_event::Event::FirstPlayerChosen(
                    proto::TicTacToeFirstPlayerChosen { x_slot },
                )),
            },
        )),
    }
}

fn tictactoe_view(view: &mg_tictactoe::View) -> proto::TicTacToeView {
    match TicTacToe::view_to_wire(view).game {
        Some(proto::game_view::Game::Tictactoe(view)) => view,
        other => panic!("not a TicTacToe view: {other:?}"),
    }
}

#[test]
fn settings_round_trip() {
    for first_player in [FirstPlayerMode::Random, FirstPlayerMode::Host] {
        let original = settings(first_player);
        let wire = TicTacToe::settings_to_wire(&original);
        assert_eq!(TicTacToe::settings_from_wire(&wire), Ok(original));
    }
}

#[test]
fn out_of_range_sizes_reach_the_game_unchanged() {
    let mut wire = TicTacToe::settings_to_wire(&Settings::default());
    if let Some(proto::game_settings::Game::Tictactoe(settings)) = wire.game.as_mut() {
        settings.field_width = u32::MAX;
    }
    let settings = TicTacToe::settings_from_wire(&wire).unwrap();
    assert_eq!(settings.field_width, u32::MAX);
    assert!(TicTacToe::new(&settings, &[PlayerSpec::HUMAN; 2]).is_err());
}

#[test]
fn commands_round_trip() {
    for point in [Point::new(0, 0), Point::new(19, 7), Point::new(u32::MAX, 3)] {
        let command = Command::Place(point);
        let wire = TicTacToe::command_to_wire(&command);
        assert_eq!(TicTacToe::command_from_wire(&wire), Ok(command));
    }
}

#[test]
fn world_events_round_trip() {
    for x_player in [Slot::new(0), Slot::new(1)] {
        let event = WorldEvent::FirstPlayerChosen { x_player };
        let wire = TicTacToe::world_event_to_wire(&event);
        assert_eq!(TicTacToe::world_event_from_wire(&wire), Ok(event));
    }
}

#[test]
fn bot_kinds_round_trip() {
    for &kind in TicTacToe::BOT_KINDS {
        let wire = TicTacToe::bot_kind_to_wire(kind);
        assert_eq!(TicTacToe::bot_kind_from_wire(&wire), Ok(kind));
    }
}

#[test]
fn malformed_settings_are_rejected() {
    assert_eq!(
        TicTacToe::settings_from_wire(&proto::GameSettings { game: None }),
        Err(WireError::Missing("game"))
    );
    assert_eq!(
        TicTacToe::settings_from_wire(&wire_settings(0)),
        Err(WireError::Missing("first_player"))
    );
    assert_eq!(
        TicTacToe::settings_from_wire(&wire_settings(99)),
        Err(WireError::UnknownEnumValue("first_player"))
    );
    let snake = proto::GameSettings {
        game: Some(proto::game_settings::Game::Snake(
            proto::SnakeSettings::default(),
        )),
    };
    assert_eq!(
        TicTacToe::settings_from_wire(&snake),
        Err(WireError::WrongGame)
    );
}

#[test]
fn malformed_commands_are_rejected() {
    assert_eq!(
        TicTacToe::command_from_wire(&proto::GameCommand { game: None }),
        Err(WireError::Missing("game"))
    );
    let empty = proto::GameCommand {
        game: Some(proto::game_command::Game::Tictactoe(
            proto::TicTacToeCommand { command: None },
        )),
    };
    assert_eq!(
        TicTacToe::command_from_wire(&empty),
        Err(WireError::Missing("command"))
    );
    let puzzle = proto::GameCommand {
        game: Some(proto::game_command::Game::Puzzle2048(
            proto::Puzzle2048Command::default(),
        )),
    };
    assert_eq!(
        TicTacToe::command_from_wire(&puzzle),
        Err(WireError::WrongGame)
    );
}

#[test]
fn malformed_world_events_are_rejected() {
    assert_eq!(
        TicTacToe::world_event_from_wire(&wire_first_player(2)),
        Err(WireError::OutOfRange("x_slot"))
    );
    assert_eq!(
        TicTacToe::world_event_from_wire(&wire_first_player(256)),
        Err(WireError::OutOfRange("slot"))
    );
    let empty = proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::Tictactoe(
            proto::TicTacToeWorldEvent { event: None },
        )),
    };
    assert_eq!(
        TicTacToe::world_event_from_wire(&empty),
        Err(WireError::Missing("event"))
    );
    let snake = proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::Snake(
            proto::SnakeWorldEvent::default(),
        )),
    };
    assert_eq!(
        TicTacToe::world_event_from_wire(&snake),
        Err(WireError::WrongGame)
    );
}

#[test]
fn malformed_bot_kinds_are_rejected() {
    let tictactoe = |kind| proto::BotKind {
        game: Some(proto::bot_kind::Game::Tictactoe(kind)),
    };
    assert_eq!(
        TicTacToe::bot_kind_from_wire(&tictactoe(0)),
        Err(WireError::Missing("bot_kind"))
    );
    assert_eq!(
        TicTacToe::bot_kind_from_wire(&tictactoe(-1)),
        Err(WireError::UnknownEnumValue("bot_kind"))
    );
    assert_eq!(
        TicTacToe::bot_kind_from_wire(&proto::BotKind { game: None }),
        Err(WireError::Missing("game"))
    );
    let snake = proto::BotKind {
        game: Some(proto::bot_kind::Game::Snake(1)),
    };
    assert_eq!(
        TicTacToe::bot_kind_from_wire(&snake),
        Err(WireError::WrongGame)
    );
}

#[test]
fn view_carries_board_players_and_end() {
    let mut game = TicTacToe::new(
        &settings(FirstPlayerMode::Host),
        &[PlayerSpec::HUMAN, PlayerSpec::HUMAN],
    )
    .unwrap();
    let before = tictactoe_view(&game.view(Viewer::Spectator));
    assert_eq!(
        (before.field_width, before.field_height, before.win_count),
        (9, 7, 4)
    );
    assert_eq!(before.cells.len(), 63);
    assert_eq!(
        (before.x_slot, before.o_slot, before.current_slot),
        (Some(0), Some(1), Some(0))
    );
    assert_eq!((before.last_move, before.end), (None, None));

    // X fills row 0 from the left, O plays row 6.
    for x in 0..4 {
        game.apply_command(Slot::new(0), &Command::Place(Point::new(x, 0)))
            .unwrap();
        if x < 3 {
            game.apply_command(Slot::new(1), &Command::Place(Point::new(x, 6)))
                .unwrap();
        }
    }
    let after = tictactoe_view(&game.view(Viewer::Player(Slot::new(1))));
    assert_eq!(after.cells.first(), Some(&i32::from(TicTacToeMark::X)));
    assert_eq!(after.cells.get(54), Some(&i32::from(TicTacToeMark::O)));
    assert_eq!(
        after.cells.get(4),
        Some(&i32::from(TicTacToeMark::Unspecified))
    );
    assert_eq!(after.current_slot, None);
    assert_eq!(after.last_move, Some(proto::TicTacToePoint { x: 3, y: 0 }));
    assert_eq!(
        game.end(),
        Some(GameEnd::Win {
            winner: Slot::new(0),
            line: WinningLine {
                mark: Mark::X,
                start: Point::new(0, 0),
                end: Point::new(3, 0),
            },
        })
    );
    assert_eq!(
        after.end,
        Some(proto::TicTacToeGameEnd {
            reason: TicTacToeEndReason::Win.into(),
            winner_slot: Some(0),
            winning_line: Some(proto::TicTacToeWinningLine {
                mark: TicTacToeMark::X.into(),
                start: Some(proto::TicTacToePoint { x: 0, y: 0 }),
                end: Some(proto::TicTacToePoint { x: 3, y: 0 }),
            }),
        })
    );
}

#[test]
fn view_reports_a_disconnect_and_an_unchosen_first_player() {
    let mut game = TicTacToe::new(
        &settings(FirstPlayerMode::Random),
        &[PlayerSpec::HUMAN, PlayerSpec::HUMAN],
    )
    .unwrap();
    let waiting = tictactoe_view(&game.view(Viewer::Spectator));
    assert_eq!(
        (waiting.x_slot, waiting.o_slot, waiting.current_slot),
        (None, None, None)
    );
    game.drop_player(Slot::new(0));
    let ended = tictactoe_view(&game.view(Viewer::Spectator));
    assert_eq!(
        ended.end,
        Some(proto::TicTacToeGameEnd {
            reason: TicTacToeEndReason::PlayerDisconnected.into(),
            winner_slot: Some(1),
            winning_line: None,
        })
    );
}

/// Every sealed frame goes through the wire (binary protobuf, and protobuf JSON with the `json`
/// feature) before a `Match` applies it, like on a real client.
fn mirror_through_the_wire(first_player: FirstPlayerMode, players: [PlayerSpec; 2], seed: u64) {
    let settings = settings(first_player);
    let wire_settings = TicTacToe::settings_to_wire(&settings);
    let bots = Slot::all(2)
        .filter(|slot| !players[slot.index()].is_human())
        .map(|slot| BotSeat {
            slot,
            bot: TicTacToe::make_bot(BotKind::Minimax),
            budget_nodes: 500,
        })
        .collect();
    let mut authority = Authority::<TicTacToe>::new(&settings, &players, seed, bots).unwrap();
    let mut mirror = Match::<TicTacToe>::new(
        &TicTacToe::settings_from_wire(&wire_settings).unwrap(),
        &players,
    )
    .unwrap();
    let mut frames = 0;
    loop {
        let frame = authority.seal_frame();
        let wire = proto::TickFrame::decode(
            frame_to_wire::<TicTacToe>(&frame)
                .encode_to_vec()
                .as_slice(),
        )
        .unwrap();
        #[cfg(feature = "json")]
        let wire: proto::TickFrame =
            serde_json::from_str(&serde_json::to_string(&wire).unwrap()).unwrap();
        mirror
            .apply_frame(&frame_from_wire::<TicTacToe>(&wire).unwrap())
            .unwrap();
        frames += 1;
        if authority.game().status().is_finished() {
            break;
        }
        match authority.game().awaiting() {
            Awaiting::Turn(slot) if players[slot.index()].is_human() => {
                let view = authority.game().view(Viewer::Player(slot));
                let empty = view.cells.iter().position(Option::is_none).unwrap();
                let width = view.field_width as usize;
                let point = Point::new((empty % width) as u32, (empty / width) as u32);
                authority.submit(slot, Command::Place(point)).unwrap();
            }
            _ => authority.run_bots(),
        }
    }
    assert!(frames > 5);
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
    assert_eq!(mirror.game().end(), authority.game().end());
}

#[test]
fn matches_replay_identically_through_the_wire() {
    for seed in 0..4 {
        mirror_through_the_wire(
            FirstPlayerMode::Random,
            [PlayerSpec::HUMAN, PlayerSpec::BOT],
            seed,
        );
        mirror_through_the_wire(
            FirstPlayerMode::Host,
            [PlayerSpec::BOT, PlayerSpec::HUMAN],
            seed,
        );
        mirror_through_the_wire(
            FirstPlayerMode::Random,
            [PlayerSpec::BOT, PlayerSpec::BOT],
            seed,
        );
    }
}

#[cfg(feature = "json")]
#[test]
fn json_uses_enum_names() {
    let json = serde_json::to_value(TicTacToe::settings_to_wire(&Settings::default())).unwrap();
    assert_eq!(
        json["tictactoe"]["firstPlayer"],
        "TIC_TAC_TOE_FIRST_PLAYER_MODE_RANDOM"
    );
    let bot = serde_json::to_value(TicTacToe::bot_kind_to_wire(BotKind::Minimax)).unwrap();
    assert_eq!(bot["tictactoe"], "TIC_TAC_TOE_BOT_KIND_MINIMAX");
}
