use mg_core::{Awaiting, CommandError, Game, PlayerSpec, Slot};

use crate::{
    CaptureRule, Checkers, Command, DrawReason, GameEnd, KING_MOVES_FOR_DRAW, Path, Rank, Refusal,
    Settings, Side, Square, WhitePlayer, WinReason, WorldEvent,
};

const HOST: Slot = Slot::new(0);
const GUEST: Slot = Slot::new(1);

/// `"c3"` → the square: files a–h left to right, ranks 1–8 from white's side.
fn sq(name: &str) -> Square {
    let bytes = name.as_bytes();
    Square::new(bytes[0] - b'a', bytes[1] - b'1').unwrap_or_else(|| panic!("{name} is light"))
}

fn path(squares: &[&str]) -> Path {
    Path::new(squares.iter().map(|name| sq(name)).collect()).unwrap()
}

fn mv(squares: &[&str]) -> Command {
    Command::Move(path(squares))
}

fn position(pieces: &[(&str, Side, Rank)], to_move: Side) -> Checkers {
    let pieces: Vec<_> = pieces
        .iter()
        .map(|(name, side, rank)| (sq(name), *side, *rank))
        .collect();
    Checkers::from_position(&pieces, to_move, 0)
}

fn paths(game: &Checkers) -> Vec<Path> {
    game.legal_moves()
        .into_iter()
        .map(|legal| legal.path)
        .collect()
}

use Rank::{King, Man};
use Side::{Black, White};

#[test]
fn white_starts_with_seven_moves_and_twelve_men_a_side() {
    let game = Checkers::new(
        &Settings {
            white_player: WhitePlayer::Host,
            capture_rule: CaptureRule::ToTheEnd,
        },
        &[PlayerSpec::HUMAN; 2],
    )
    .unwrap();

    assert_eq!(game.board().count(White), 12);
    assert_eq!(game.board().count(Black), 12);
    assert_eq!(game.awaiting(), Awaiting::Turn(HOST));
    assert_eq!(game.legal_moves().len(), 7);
}

#[test]
fn a_man_steps_one_square_diagonally_forward_only() {
    let game = position(&[("d4", White, Man), ("h8", Black, Man)], White);

    assert_eq!(paths(&game), vec![path(&["d4", "e5"]), path(&["d4", "c5"])]);
}

#[test]
fn capturing_is_compulsory() {
    let mut game = position(
        &[("c3", White, Man), ("d4", Black, Man), ("g3", White, Man)],
        White,
    );

    let refused = game.apply_command(HOST, &mv(&["g3", "h4"]));

    assert_eq!(refused, Err(CommandError::Illegal(Refusal::MustCapture)));
    assert_eq!(paths(&game), vec![path(&["c3", "e5"])]);
}

#[test]
fn a_man_captures_backwards_too() {
    let game = position(&[("e5", White, Man), ("d4", Black, Man)], White);

    assert_eq!(paths(&game), vec![path(&["e5", "c3"])]);
}

#[test]
fn a_capture_goes_on_while_it_can() {
    let mut game = position(
        &[("c1", White, Man), ("d2", Black, Man), ("d4", Black, Man)],
        White,
    );

    assert!(game.apply_command(HOST, &mv(&["c1", "e3"])).is_err());
    game.apply_command(HOST, &mv(&["c1", "e3", "c5"])).unwrap();

    assert_eq!(game.board().count(Black), 0);
}

#[test]
fn any_capture_sequence_will_do_not_only_the_longest() {
    let game = position(
        &[
            ("c3", White, Man),
            ("b4", Black, Man),
            ("d4", Black, Man),
            ("f6", Black, Man),
        ],
        White,
    );

    let legal = paths(&game);

    assert!(legal.contains(&path(&["c3", "a5"])));
    assert!(legal.contains(&path(&["c3", "e5", "g7"])));
}

#[test]
fn a_king_flies_and_lands_anywhere_beyond_the_piece_it_takes() {
    let game = position(&[("a1", White, King), ("d4", Black, Man)], White);

    assert_eq!(
        paths(&game),
        vec![
            path(&["a1", "e5"]),
            path(&["a1", "f6"]),
            path(&["a1", "g7"]),
            path(&["a1", "h8"]),
        ]
    );
}

#[test]
fn a_king_must_land_where_it_can_capture_again() {
    let game = position(
        &[("a1", White, King), ("d4", Black, Man), ("f4", Black, Man)],
        White,
    );

    assert_eq!(
        paths(&game),
        vec![path(&["a1", "e5", "g3"]), path(&["a1", "e5", "h2"])]
    );
}

#[test]
fn a_man_crowned_in_the_middle_of_a_capture_goes_on_as_a_king() {
    let mut game = position(
        &[("b6", White, Man), ("c7", Black, Man), ("f6", Black, Man)],
        White,
    );

    game.apply_command(HOST, &mv(&["b6", "d8", "g5"])).unwrap();

    assert_eq!(
        game.board().at(sq("g5")).map(|piece| piece.rank),
        Some(King)
    );
    assert_eq!(game.board().count(Black), 0);
}

#[test]
fn a_man_reaching_the_far_row_becomes_a_king() {
    let mut game = position(&[("a7", White, Man), ("h2", Black, Man)], White);

    game.apply_command(HOST, &mv(&["a7", "b8"])).unwrap();

    assert_eq!(
        game.board().at(sq("b8")).map(|piece| piece.rank),
        Some(King)
    );
}

#[test]
fn a_captured_piece_blocks_until_the_move_is_over() {
    // The king takes d4 and lands on e5: from there f6 is behind the d4 piece only if d4 were
    // gone. It still stands, so the king cannot jump it again on the way back.
    let game = position(
        &[("a1", White, King), ("d4", Black, Man), ("c5", Black, Man)],
        White,
    );

    for legal in game.legal_moves() {
        let mut captured = legal.captured.clone();
        captured.sort();
        captured.dedup();
        assert_eq!(captured.len(), legal.captured.len(), "{legal:?}");
    }
}

#[test]
fn taking_the_last_piece_wins() {
    let mut game = position(&[("c3", White, Man), ("d4", Black, Man)], White);

    game.apply_command(HOST, &mv(&["c3", "e5"])).unwrap();

    assert_eq!(
        game.end(),
        Some(GameEnd::Win {
            winner: HOST,
            reason: WinReason::NoPieces
        })
    );
}

#[test]
fn a_side_that_cannot_move_loses() {
    // Black's man on b2 is boxed in: every square it could step or jump to is taken.
    let mut game = position(
        &[
            ("b2", Black, Man),
            ("a1", White, Man),
            ("c1", White, Man),
            ("a3", White, Man),
            ("c3", White, Man),
            ("d4", White, Man),
            ("g1", White, Man),
        ],
        White,
    );

    game.apply_command(HOST, &mv(&["g1", "h2"])).unwrap();

    assert_eq!(
        game.end(),
        Some(GameEnd::Win {
            winner: HOST,
            reason: WinReason::NoMoves
        })
    );
}

#[test]
fn fifteen_king_moves_each_without_a_capture_draw() {
    let pieces = [(sq("c1"), White, King), (sq("f8"), Black, King)];
    let mut game = Checkers::from_position(&pieces, White, KING_MOVES_FOR_DRAW - 1);

    game.apply_command(HOST, &mv(&["c1", "d2"])).unwrap();

    assert_eq!(game.end(), Some(GameEnd::Draw(DrawReason::KingMoves)));
}

#[test]
fn a_man_moving_starts_the_king_move_count_afresh() {
    let pieces = [
        (sq("c1"), White, King),
        (sq("g1"), White, Man),
        (sq("f8"), Black, King),
    ];
    let mut game = Checkers::from_position(&pieces, White, KING_MOVES_FOR_DRAW - 1);

    game.apply_command(HOST, &mv(&["g1", "h2"])).unwrap();

    assert_eq!(game.end(), None);
}

#[test]
fn the_same_position_a_third_time_draws() {
    let mut game = position(&[("c1", White, King), ("f8", Black, King)], White);
    let shuffle = [
        (HOST, ["c1", "d2"]),
        (GUEST, ["f8", "e7"]),
        (HOST, ["d2", "c1"]),
        (GUEST, ["e7", "f8"]),
    ];

    for (slot, squares) in shuffle {
        game.apply_command(slot, &mv(&squares)).unwrap();
    }
    assert_eq!(game.end(), None, "twice is not yet a draw");
    for (slot, squares) in shuffle {
        game.apply_command(slot, &mv(&squares)).unwrap();
    }

    assert_eq!(game.end(), Some(GameEnd::Draw(DrawReason::Repetition)));
}

#[test]
fn leaving_loses() {
    let mut game = position(&[("c1", White, King), ("f8", Black, King)], White);

    game.drop_player(HOST);

    assert_eq!(
        game.end(),
        Some(GameEnd::Win {
            winner: GUEST,
            reason: WinReason::OpponentLeft
        })
    );
}

#[test]
fn white_is_drawn_by_lot_once() {
    let mut game = Checkers::new(&Settings::default(), &[PlayerSpec::HUMAN; 2]).unwrap();
    assert_eq!(game.awaiting(), Awaiting::Nobody);

    game.apply_world(&WorldEvent::WhiteChosen(GUEST)).unwrap();

    assert_eq!(game.awaiting(), Awaiting::Turn(GUEST));
    assert_eq!(game.side_of(GUEST), Some(White));
    assert!(game.apply_world(&WorldEvent::WhiteChosen(HOST)).is_err());
}

#[test]
fn only_the_side_to_move_moves() {
    let mut game = position(&[("c1", White, King), ("f8", Black, King)], White);

    let refused = game.apply_command(GUEST, &mv(&["f8", "e7"]));

    assert_eq!(refused, Err(CommandError::NotYourTurn));
}

#[test]
fn a_capture_must_go_on_to_the_end_by_default() {
    let game = position(
        &[("c3", White, Man), ("d4", Black, Man), ("f6", Black, Man)],
        White,
    );

    assert_eq!(paths(&game), vec![path(&["c3", "e5", "g7"])]);
}

#[test]
fn under_the_house_rule_a_capture_may_stop_after_any_jump() {
    let game = position(
        &[("c3", White, Man), ("d4", Black, Man), ("f6", Black, Man)],
        White,
    )
    .with_capture_rule(CaptureRule::MayStop);

    assert_eq!(
        paths(&game),
        vec![path(&["c3", "e5"]), path(&["c3", "e5", "g7"])]
    );
}

#[test]
fn a_king_lands_where_it_can_go_on_by_default_anywhere_under_the_house_rule() {
    let pieces = [("a1", White, King), ("b2", Black, Man), ("f2", Black, Man)];

    let strict = position(&pieces, White);
    assert_eq!(paths(&strict), vec![path(&["a1", "d4", "g1"])]);

    let free = position(&pieces, White).with_capture_rule(CaptureRule::MayStop);
    let mut moves = paths(&free);
    moves.sort_by_key(|moved| {
        moved
            .squares()
            .iter()
            .map(|square| square.index())
            .collect::<Vec<_>>()
    });
    let mut expected = vec![
        path(&["a1", "c3"]),
        path(&["a1", "d4"]),
        path(&["a1", "d4", "g1"]),
        path(&["a1", "e5"]),
        path(&["a1", "f6"]),
        path(&["a1", "g7"]),
        path(&["a1", "h8"]),
    ];
    expected.sort_by_key(|moved| {
        moved
            .squares()
            .iter()
            .map(|square| square.index())
            .collect::<Vec<_>>()
    });
    assert_eq!(moves, expected);
}

#[test]
fn stopping_early_under_the_house_rule_takes_only_the_jumped_pieces() {
    let mut game = position(
        &[("c3", White, Man), ("d4", Black, Man), ("f6", Black, Man)],
        White,
    )
    .with_capture_rule(CaptureRule::MayStop);

    game.apply_command(HOST, &mv(&["c3", "e5"])).unwrap();

    assert!(game.board().at(sq("d4")).is_none());
    assert!(game.board().at(sq("f6")).is_some());
    assert_eq!(
        game.board().at(sq("e5")).map(|piece| piece.side),
        Some(White)
    );
}
