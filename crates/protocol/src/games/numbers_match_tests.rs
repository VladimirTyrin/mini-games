use mg_core::{Authority, CoreRng, Game, GameFrame, Match, PlayerSpec, Slot, Viewer};
use mg_numbers_match::{
    CellIndex, CellPair, Command, HintMode, MAX_REFILLS, NumbersMatch, Phase, Settings, WorldEvent,
};
use prost::Message;

use crate::proto;
use crate::{WireError, WireGame, frame_from_wire, frame_to_wire};

const PLAYER: Slot = Slot::new(0);

fn dealt_game(settings: Settings, seed: u64) -> NumbersMatch {
    let mut game = NumbersMatch::new(&settings, &[PlayerSpec::HUMAN]).unwrap();
    for event in game.roll_world(&mut CoreRng::new(seed)) {
        game.apply_world(&event).unwrap();
    }
    game
}

fn wire_command(command: proto::numbers_match_command::Command) -> proto::GameCommand {
    proto::GameCommand {
        game: Some(proto::game_command::Game::NumbersMatch(
            proto::NumbersMatchCommand {
                command: Some(command),
            },
        )),
    }
}

fn wire_deal(values: Vec<u32>) -> proto::GameWorldEvent {
    proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::NumbersMatch(
            proto::NumbersMatchWorldEvent {
                event: Some(proto::numbers_match_world_event::Event::BoardDealt(
                    proto::NumbersMatchBoardDealt { values },
                )),
            },
        )),
    }
}

fn wire_settings(hint_mode: i32) -> proto::GameSettings {
    proto::GameSettings {
        game: Some(proto::game_settings::Game::NumbersMatch(
            proto::NumbersMatchSettings {
                hint_mode,
                refills: 5,
            },
        )),
    }
}

fn numbers_match_view(view: proto::GameView) -> proto::NumbersMatchView {
    match view.game {
        Some(proto::game_view::Game::NumbersMatch(view)) => view,
        other => panic!("not a Numbers Match view: {other:?}"),
    }
}

#[test]
fn settings_round_trip() {
    for hint_mode in [HintMode::Limited, HintMode::Unlimited, HintMode::Disabled] {
        for refills in [0, 5, MAX_REFILLS] {
            let settings = Settings { hint_mode, refills };
            let wire = NumbersMatch::settings_to_wire(&settings);
            assert_eq!(NumbersMatch::settings_from_wire(&wire), Ok(settings));
        }
    }
}

#[test]
fn commands_round_trip() {
    let pair = CellPair::new(CellIndex::new(44), CellIndex::new(3));
    for command in [
        Command::RemovePair(pair),
        Command::Refill,
        Command::RequestHint,
    ] {
        let wire = NumbersMatch::command_to_wire(&command);
        assert_eq!(NumbersMatch::command_from_wire(&wire), Ok(command));
    }
}

#[test]
fn deal_round_trips() {
    let game = NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    for event in game.roll_world(&mut CoreRng::new(9)) {
        let wire = NumbersMatch::world_event_to_wire(&event);
        assert_eq!(NumbersMatch::world_event_from_wire(&wire), Ok(event));
    }
}

/// Plays a full match on an authority (hints, refills, removals), ships every sealed frame
/// through `transport` and replays it on a `Match`.
fn replay_through(transport: impl Fn(proto::TickFrame) -> proto::TickFrame) {
    let settings = Settings {
        hint_mode: HintMode::Unlimited,
        ..Settings::default()
    };
    let players = [PlayerSpec::HUMAN];
    let mut authority = Authority::<NumbersMatch>::new(&settings, &players, 7, vec![]).unwrap();
    let mut mirror = Match::<NumbersMatch>::new(&settings, &players).unwrap();
    let mut sealed = 0;
    loop {
        let frame: GameFrame<NumbersMatch> = authority.seal_frame();
        let wire = transport(frame_to_wire::<NumbersMatch>(&frame));
        let received = frame_from_wire::<NumbersMatch>(&wire).unwrap();
        assert_eq!(received, frame);
        mirror.apply_frame(&received).unwrap();
        sealed += 1;
        let game = authority.game();
        if game.phase().is_over() {
            break;
        }
        let command = match game.find_pair() {
            Some(_) if game.hint().is_none() && sealed % 5 == 0 => Command::RequestHint,
            Some(pair) => Command::RemovePair(CellPair::new(pair.second, pair.first)),
            None => Command::Refill,
        };
        authority.submit(PLAYER, command).unwrap();
    }
    assert!(sealed > 20, "{sealed}");
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
}

#[test]
fn match_replays_identically_through_binary_protobuf() {
    replay_through(|frame| proto::TickFrame::decode(frame.encode_to_vec().as_slice()).unwrap());
}

#[cfg(feature = "json")]
#[test]
fn match_replays_identically_through_json() {
    replay_through(|frame| {
        let json = serde_json::to_string(&frame).unwrap();
        serde_json::from_str(&json).unwrap()
    });
}

#[test]
fn view_carries_hint_and_last_action() {
    let settings = Settings {
        hint_mode: HintMode::Unlimited,
        ..Settings::default()
    };
    let mut game = dealt_game(settings, 3);
    game.apply_command(PLAYER, &Command::RequestHint).unwrap();
    let pair = game.find_pair().unwrap();

    let view = numbers_match_view(NumbersMatch::view_to_wire(&game.view(Viewer::Spectator)));
    let expected_pair = proto::NumbersMatchCellPair {
        first_index: pair.first.get(),
        second_index: pair.second.get(),
    };
    assert_eq!(
        view.hint.and_then(|hint| hint.hint),
        Some(proto::numbers_match_hint::Hint::Pair(expected_pair))
    );
    assert_eq!(view.hints_left, None);
    assert_eq!(view.columns, 9);
    assert_eq!(view.cells.len() as u32, view.rows * view.columns);

    game.apply_command(PLAYER, &Command::RemovePair(pair))
        .unwrap();
    let view = numbers_match_view(NumbersMatch::view_to_wire(&game.view(Viewer::Spectator)));
    assert_eq!(view.hint, None);
    assert_eq!(view.phase(), proto::NumbersMatchPhase::Playing);
    let removed = proto::NumbersMatchPairRemoved {
        pair: Some(expected_pair),
        deleted_rows: vec![],
    };
    assert_eq!(
        view.last_action.and_then(|action| action.action),
        Some(proto::numbers_match_last_action::Action::PairRemoved(
            removed
        ))
    );
    assert_eq!(view.stats.map(|stats| stats.pairs_removed), Some(1));
    let removed_cells = view
        .cells
        .iter()
        .filter(|cell| cell.state() == proto::NumbersMatchCellState::Removed)
        .count();
    assert_eq!(removed_cells, 2);
}

#[test]
fn view_reports_why_the_game_was_lost() {
    let mut game = dealt_game(Settings::default(), 1);
    game.drop_player(PLAYER);
    assert_eq!(
        game.phase(),
        Phase::Lost(mg_numbers_match::LossReason::Dropped)
    );
    let view = numbers_match_view(NumbersMatch::view_to_wire(&game.view(Viewer::Spectator)));
    assert_eq!(view.phase(), proto::NumbersMatchPhase::Lost);
    assert_eq!(view.loss_reason(), proto::NumbersMatchLossReason::Dropped);
    assert_eq!(view.hints_left, Some(3));
}

#[cfg(feature = "json")]
#[test]
fn view_json_uses_renderer_friendly_names() {
    let game = dealt_game(Settings::default(), 1);
    let view = NumbersMatch::view_to_wire(&game.view(Viewer::Spectator));
    let json = serde_json::to_value(&view).unwrap();
    let view = &json["numbersMatch"];
    assert_eq!(view["phase"], "NUMBERS_MATCH_PHASE_PLAYING");
    assert_eq!(view["hintsLeft"], 3);
    assert_eq!(view["cells"][0]["state"], "NUMBERS_MATCH_CELL_STATE_ACTIVE");
}

#[test]
fn malformed_settings_are_rejected() {
    assert_eq!(
        NumbersMatch::settings_from_wire(&proto::GameSettings { game: None }),
        Err(WireError::Missing("game"))
    );
    assert_eq!(
        NumbersMatch::settings_from_wire(&wire_settings(0)),
        Err(WireError::Missing("hint_mode"))
    );
    assert_eq!(
        NumbersMatch::settings_from_wire(&wire_settings(99)),
        Err(WireError::UnknownEnumValue("hint_mode"))
    );
    let snake = proto::GameSettings {
        game: Some(proto::game_settings::Game::Snake(Default::default())),
    };
    assert_eq!(
        NumbersMatch::settings_from_wire(&snake),
        Err(WireError::WrongGame)
    );
}

#[test]
fn malformed_commands_are_rejected() {
    let empty = proto::GameCommand {
        game: Some(proto::game_command::Game::NumbersMatch(
            proto::NumbersMatchCommand { command: None },
        )),
    };
    assert_eq!(
        NumbersMatch::command_from_wire(&empty),
        Err(WireError::Missing("command"))
    );
    assert_eq!(
        NumbersMatch::command_from_wire(&proto::GameCommand { game: None }),
        Err(WireError::Missing("game"))
    );
    let tictactoe = proto::GameCommand {
        game: Some(proto::game_command::Game::Tictactoe(Default::default())),
    };
    assert_eq!(
        NumbersMatch::command_from_wire(&tictactoe),
        Err(WireError::WrongGame)
    );
    // Indices are not range-checked on the wire; the game rejects them.
    let far_away = wire_command(proto::numbers_match_command::Command::RemovePair(
        proto::NumbersMatchCellPair {
            first_index: u32::MAX,
            second_index: 0,
        },
    ));
    let command = NumbersMatch::command_from_wire(&far_away).unwrap();
    let mut game = dealt_game(Settings::default(), 1);
    assert!(game.apply_command(PLAYER, &command).is_err());
}

#[test]
fn malformed_deals_are_rejected() {
    for bad_digit in [0, 10] {
        let mut values = vec![1; 42];
        values[17] = bad_digit;
        assert_eq!(
            NumbersMatch::world_event_from_wire(&wire_deal(values)),
            Err(WireError::OutOfRange("values"))
        );
    }
    for too_wide in [256 + 5, u32::MAX] {
        let mut values = vec![1; 42];
        values[17] = too_wide;
        assert_eq!(
            NumbersMatch::world_event_from_wire(&wire_deal(values)),
            Err(WireError::IntegerOutOfRange {
                field: "values",
                source: u8::try_from(too_wide).unwrap_err(),
            })
        );
    }
    let no_event = proto::GameWorldEvent {
        game: Some(proto::game_world_event::Game::NumbersMatch(
            proto::NumbersMatchWorldEvent { event: None },
        )),
    };
    assert_eq!(
        NumbersMatch::world_event_from_wire(&no_event),
        Err(WireError::Missing("event"))
    );
    // A well-formed but illegal deal decodes, and the game rejects it.
    let short = NumbersMatch::world_event_from_wire(&wire_deal(vec![1, 2, 3])).unwrap();
    assert!(matches!(&short, WorldEvent::BoardDealt { values } if values.len() == 3));
    let mut game = NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    assert!(game.apply_world(&short).is_err());
}
