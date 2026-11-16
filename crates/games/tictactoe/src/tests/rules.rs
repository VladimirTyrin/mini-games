use mg_core::{
    Authority, Awaiting, CommandError, Game, Input, OutcomeKind, PlayerSpec, SetupError, Status,
    Viewer,
};

use super::{GUEST, HOST, new_game, place, settings};
use crate::{
    Command, FirstPlayerMode, GameEnd, Mark, Point, Settings, TicTacToe, WinningLine, WorldEvent,
};

const HUMANS: [PlayerSpec; 2] = [PlayerSpec::HUMAN, PlayerSpec::HUMAN];

fn line(mark: Mark, start: (u32, u32), end: (u32, u32)) -> WinningLine {
    WinningLine {
        mark,
        start: Point::new(start.0, start.1),
        end: Point::new(end.0, end.1),
    }
}

fn winning_line(game: &TicTacToe) -> Option<WinningLine> {
    match game.end() {
        Some(GameEnd::Win { line, .. }) => Some(line),
        _ => None,
    }
}

#[test]
fn defaults_match_the_web_client() {
    assert_eq!(
        Settings::default(),
        Settings {
            field_width: 15,
            field_height: 15,
            win_count: 5,
            first_player: FirstPlayerMode::Random,
        }
    );
    assert!(TicTacToe::new(&Settings::default(), &HUMANS).is_ok());
}

#[test]
fn settings_out_of_range_are_rejected() {
    let rejected = [
        settings(2, 15, 3),
        settings(21, 15, 5),
        settings(15, 2, 3),
        settings(15, 21, 5),
        settings(0, 0, 0),
        settings(u32::MAX, u32::MAX, 5),
        settings(15, 15, 2),
        settings(15, 15, 16),
        settings(4, 7, 5),
        settings(15, 15, u32::MAX),
    ];
    for settings in rejected {
        assert!(
            matches!(
                TicTacToe::new(&settings, &HUMANS),
                Err(SetupError::InvalidSetting { .. })
            ),
            "{settings:?}"
        );
    }
    for edge in [settings(3, 3, 3), settings(20, 20, 20), settings(3, 20, 3)] {
        assert!(TicTacToe::new(&edge, &HUMANS).is_ok(), "{edge:?}");
    }
}

#[test]
fn exactly_two_seats_are_required() {
    for seats in [0, 1, 3] {
        let players = vec![PlayerSpec::HUMAN; seats];
        assert!(matches!(
            TicTacToe::new(&Settings::default(), &players),
            Err(SetupError::PlayerCount { got, .. }) if got == seats
        ));
    }
}

#[test]
fn x_and_o_alternate() {
    let mut game = new_game(3, 3, 3);
    assert_eq!(game.awaiting(), Awaiting::Turn(HOST));
    place(&mut game, 0, 0);
    assert_eq!(game.awaiting(), Awaiting::Turn(GUEST));
    place(&mut game, 1, 1);
    let view = game.view(Viewer::Spectator);
    assert_eq!(view.cells.first(), Some(&Some(Mark::X)));
    assert_eq!(view.cells.get(4), Some(&Some(Mark::O)));
    assert_eq!(view.last_move, Some(Point::new(1, 1)));
    assert_eq!(view.current_player, Some(HOST));
}

#[test]
fn illegal_commands_are_rejected_without_changing_state() {
    let mut game = new_game(3, 3, 3);
    place(&mut game, 1, 1);
    let before = game.state_hash();
    let attempts = [
        (HOST, Point::new(0, 0), CommandError::NotYourTurn),
        (
            GUEST,
            Point::new(1, 1),
            CommandError::Illegal("cell is already marked"),
        ),
        (
            GUEST,
            Point::new(3, 0),
            CommandError::Illegal("cell is outside the board"),
        ),
        (
            GUEST,
            Point::new(u32::MAX, u32::MAX),
            CommandError::Illegal("cell is outside the board"),
        ),
        (
            mg_core::Slot::new(2),
            Point::new(0, 0),
            CommandError::NotPlaying,
        ),
    ];
    for (slot, point, expected) in attempts {
        assert_eq!(
            game.apply_command(slot, &Command::Place(point)),
            Err(expected)
        );
        assert_eq!(game.state_hash(), before);
    }
}

#[test]
fn wins_in_all_four_directions_report_the_line() {
    // X moves along the line, O plays harmless moves in the last row.
    let cases = [
        ([(1, 2), (2, 2), (3, 2)], line(Mark::X, (1, 2), (3, 2))),
        ([(4, 1), (4, 2), (4, 3)], line(Mark::X, (4, 1), (4, 3))),
        ([(1, 1), (2, 2), (3, 3)], line(Mark::X, (1, 1), (3, 3))),
        ([(1, 3), (2, 2), (3, 1)], line(Mark::X, (3, 1), (1, 3))),
    ];
    for (xs, expected) in cases {
        let mut game = new_game(6, 6, 3);
        for (turn, (x, y)) in xs.into_iter().enumerate() {
            place(&mut game, x, y);
            if game.end().is_none() {
                place(&mut game, 2 * turn as u32, 5);
            }
        }
        assert_eq!(
            game.end(),
            Some(GameEnd::Win {
                winner: HOST,
                line: expected
            })
        );
        assert_eq!(game.awaiting(), Awaiting::Nobody);
        assert_eq!(
            game.status(),
            Status::Finished(mg_core::Outcome {
                kind: OutcomeKind::Winner,
                winners: vec![HOST],
                scores: vec![1, 0],
            })
        );
        assert_eq!(game.view(Viewer::Spectator).current_player, None);
    }
}

#[test]
fn o_can_win_and_the_line_spans_the_whole_run() {
    let mut game = new_game(7, 7, 3);
    // O builds (0,1) and (2,1), then joins them through (1,1); X never threatens.
    for (x_move, o_move) in [((6, 6), (0, 1)), ((6, 4), (2, 1))] {
        place(&mut game, x_move.0, x_move.1);
        place(&mut game, o_move.0, o_move.1);
    }
    place(&mut game, 4, 4);
    place(&mut game, 1, 1);
    assert_eq!(
        game.end(),
        Some(GameEnd::Win {
            winner: GUEST,
            line: line(Mark::O, (0, 1), (2, 1))
        })
    );

    // An overline (longer than win_count) wins and is reported whole.
    let mut game = new_game(7, 7, 3);
    for (x_move, o_move) in [((0, 0), (0, 6)), ((1, 0), (2, 6)), ((3, 0), (4, 6))] {
        place(&mut game, x_move.0, x_move.1);
        place(&mut game, o_move.0, o_move.1);
    }
    place(&mut game, 4, 0);
    assert_eq!(game.end(), None, "two separate pairs");
    place(&mut game, 6, 6);
    place(&mut game, 2, 0);
    assert_eq!(winning_line(&game), Some(line(Mark::X, (0, 0), (4, 0))));
}

#[test]
fn win_count_needs_that_many_in_a_row() {
    let mut game = new_game(5, 5, 4);
    for x in 0..3 {
        place(&mut game, x, 0);
        place(&mut game, x, 4);
    }
    assert_eq!(game.end(), None);
    place(&mut game, 3, 0);
    assert_eq!(winning_line(&game), Some(line(Mark::X, (0, 0), (3, 0))));
}

#[test]
fn full_board_without_a_line_is_a_draw() {
    let mut game = new_game(3, 3, 3);
    // X O X / X O O / O X X
    for (x, y) in [
        (0, 0),
        (1, 0),
        (2, 0),
        (1, 1),
        (0, 1),
        (0, 2),
        (1, 2),
        (2, 1),
        (2, 2),
    ] {
        place(&mut game, x, y);
    }
    assert_eq!(game.end(), Some(GameEnd::Draw));
    assert_eq!(
        game.status(),
        Status::Finished(mg_core::Outcome {
            kind: OutcomeKind::Draw,
            winners: vec![],
            scores: vec![0, 0],
        })
    );
    assert_eq!(
        game.apply_command(GUEST, &Command::Place(Point::new(0, 0))),
        Err(CommandError::GameOver)
    );
}

#[test]
fn a_win_on_the_last_cell_is_a_win_not_a_draw() {
    let mut game = new_game(3, 3, 3);
    // X O X / O X O / O X X(last)
    for (x, y) in [
        (0, 0),
        (1, 0),
        (2, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (1, 2),
        (0, 2),
        (2, 2),
    ] {
        place(&mut game, x, y);
    }
    assert_eq!(winning_line(&game), Some(line(Mark::X, (0, 0), (2, 2))));
}

/// T1/T2: legacy "Host" depended on HashMap order and replays always assumed Random.
#[test]
fn host_mode_gives_the_host_x_and_the_first_move_without_randomness() {
    let host_mode = settings(15, 15, 5);
    for seed in 0..20 {
        let mut authority = Authority::<TicTacToe>::new(&host_mode, &HUMANS, seed, vec![]).unwrap();
        let frame = authority.seal_frame();
        assert!(frame.inputs.is_empty());
        let view = authority.game().view(Viewer::Spectator);
        assert_eq!(view.x_player, Some(HOST));
        assert_eq!(view.o_player, Some(GUEST));
        assert_eq!(authority.game().awaiting(), Awaiting::Turn(HOST));
    }
}

#[test]
fn random_mode_announces_the_first_player_as_a_world_event() {
    let random_mode = Settings::default();
    let mut chosen = Vec::new();
    for seed in 0..32 {
        let mut authority =
            Authority::<TicTacToe>::new(&random_mode, &HUMANS, seed, vec![]).unwrap();
        assert_eq!(authority.game().awaiting(), Awaiting::Nobody);
        assert_eq!(
            authority.submit(HOST, Command::Place(Point::new(7, 7))),
            Err(CommandError::NotYourTurn)
        );
        let frame = authority.seal_frame();
        let [Input::World(WorldEvent::FirstPlayerChosen { x_player })] = frame.inputs[..] else {
            panic!("expected one FirstPlayerChosen, got {:?}", frame.inputs);
        };
        assert_eq!(
            authority.game().view(Viewer::Spectator).x_player,
            Some(x_player)
        );
        assert_eq!(authority.game().awaiting(), Awaiting::Turn(x_player));
        assert!(authority.seal_frame().inputs.is_empty(), "chosen only once");
        chosen.push(x_player);
    }
    assert!(chosen.contains(&HOST) && chosen.contains(&GUEST));
}

#[test]
fn first_player_event_is_rejected_when_not_expected() {
    let mut host_mode = new_game(3, 3, 3);
    let event = WorldEvent::FirstPlayerChosen { x_player: GUEST };
    assert!(host_mode.apply_world(&event).is_err());

    let mut random_mode = TicTacToe::new(&Settings::default(), &HUMANS).unwrap();
    let before = random_mode.state_hash();
    let bad_seat = WorldEvent::FirstPlayerChosen {
        x_player: mg_core::Slot::new(2),
    };
    assert!(random_mode.apply_world(&bad_seat).is_err());
    assert_eq!(random_mode.state_hash(), before);
    random_mode.apply_world(&event).unwrap();
    assert!(random_mode.apply_world(&event).is_err());
}

/// T8: legacy reported a plain win when a player left.
#[test]
fn leaving_hands_the_win_to_the_opponent_with_its_own_reason() {
    let mut game = new_game(15, 15, 5);
    place(&mut game, 7, 7);
    game.drop_player(HOST);
    assert_eq!(game.end(), Some(GameEnd::OpponentLeft { winner: GUEST }));
    assert_eq!(game.awaiting(), Awaiting::Nobody);
    assert!(matches!(game.status(), Status::Finished(outcome) if outcome.winners == vec![GUEST]));

    let after_end = game.state_hash();
    game.drop_player(GUEST);
    assert_eq!(
        game.state_hash(),
        after_end,
        "a finished game stays finished"
    );
}

#[test]
fn leaving_before_the_first_player_is_chosen_also_forfeits() {
    let mut game = TicTacToe::new(&Settings::default(), &HUMANS).unwrap();
    game.drop_player(GUEST);
    assert_eq!(game.end(), Some(GameEnd::OpponentLeft { winner: HOST }));
    let mut rng = mg_core::CoreRng::new(1);
    assert!(game.roll_world(&mut rng).is_empty());
}

#[test]
fn view_carries_board_geometry_and_players() {
    let game = new_game(4, 7, 4);
    let view = game.view(Viewer::Player(GUEST));
    assert_eq!(
        (view.field_width, view.field_height, view.win_count),
        (4, 7, 4)
    );
    assert_eq!(view.cells.len(), 28);
    assert!(view.cells.iter().all(Option::is_none));
    assert_eq!(view.end, None);
    assert_eq!(game.mark_of(HOST), Some(Mark::X));
    assert_eq!(game.mark_of(GUEST), Some(Mark::O));
    assert_eq!(game.player_with(Mark::O), Some(GUEST));
}
