use mg_core::{Bot, Budget, CoreRng, Game, Slot};

use super::{GUEST, HOST, decide, new_game, place, position};
use crate::bots::{Candidate, Neighbourhood, Score, Search, SearchBoard, Side};
use crate::{Command, GameEnd, MINIMAX_BOT_BUDGET, Mark, MinimaxBot, Point, RandomBot, TicTacToe};

/// Small budget so debug-build tests stay fast; strength tests use the default.
const TEST_BUDGET: u64 = 1_000_000;

fn chebyshev(a: Point, b: Point) -> u32 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

fn occupied(game: &TicTacToe) -> Vec<Point> {
    let view = game.view(mg_core::Viewer::Spectator);
    (0..view.field_height)
        .flat_map(|y| (0..view.field_width).map(move |x| Point::new(x, y)))
        .filter(|point| game.board().mark_at(*point).is_some())
        .collect()
}

/// Plays `moves` random legal moves (fewer if the game ends first).
fn random_position(size: u32, win_count: u32, moves: usize, rng: &mut CoreRng) -> TicTacToe {
    let mut game = new_game(size, size, win_count);
    for _ in 0..moves {
        if game.end().is_some() {
            break;
        }
        let empty: Vec<Point> = (0..size)
            .flat_map(|y| (0..size).map(move |x| Point::new(x, y)))
            .filter(|point| game.board().mark_at(*point).is_none())
            .collect();
        let point = *rng.pick(&empty).unwrap();
        place(&mut game, point.x, point.y);
    }
    game
}

fn play_out(
    game: &mut TicTacToe,
    seats: [&mut dyn Bot<TicTacToe>; 2],
    budgets: [u64; 2],
    seed: u64,
) -> GameEnd {
    let mut rngs = [CoreRng::derive(seed, 1), CoreRng::derive(seed, 2)];
    while game.end().is_none() {
        let slot = game.current_player().unwrap();
        let seat = slot.index();
        let mut budget = Budget::new(budgets[seat]);
        let command = seats[seat]
            .decide(game, slot, &mut rngs[seat], &mut budget)
            .expect("bot must move on its turn");
        game.apply_command(slot, &command).unwrap();
    }
    game.end().unwrap()
}

#[test]
fn bots_start_in_the_centre() {
    for size in [3, 4, 15, 20] {
        let game = new_game(size, size, 3);
        let centre = Point::new(size / 2, size / 2);
        assert_eq!(decide(&mut RandomBot, &game, 1), centre);
        assert_eq!(decide(&mut MinimaxBot, &game, TEST_BUDGET), centre);
    }
}

#[test]
fn random_bot_plays_empty_cells_within_two_of_a_mark() {
    let game = position(15, 5, &[(0, 0), (9, 9)], &[(14, 14)]);
    let marks = occupied(&game);
    let mut seen = Vec::new();
    for seed in 0..300 {
        let mut rng = CoreRng::new(seed);
        let Some(Command::Place(point)) =
            RandomBot.decide(&game, GUEST, &mut rng, &mut Budget::new(1))
        else {
            panic!("no move");
        };
        assert!(game.board().mark_at(point).is_none());
        assert!(
            marks.iter().any(|mark| chebyshev(*mark, point) <= 2),
            "{point:?}"
        );
        if !seen.contains(&point) {
            seen.push(point);
        }
    }
    // 8 + 24 + 8 empty cells around the three marks; uniform picks reach most of them.
    assert!(seen.len() > 30, "{}", seen.len());
}

#[test]
fn bots_do_not_move_out_of_turn() {
    let game = new_game(3, 3, 3);
    let mut rng = CoreRng::new(1);
    assert_eq!(
        MinimaxBot.decide(&game, GUEST, &mut rng, &mut Budget::new(100)),
        None
    );
    assert_eq!(
        RandomBot.decide(&game, GUEST, &mut rng, &mut Budget::new(100)),
        None
    );
}

#[test]
fn bots_always_return_a_legal_move() {
    let mut rng = CoreRng::new(99);
    for round in 0..72_u32 {
        let size = 3 + round % 18;
        let win_count = 3 + round % (size - 2);
        let cells = (size * size) as usize;
        let moves = rng.index(cells).unwrap();
        let game = random_position(size, win_count, moves, &mut rng);
        let Some(slot) = game.current_player() else {
            continue;
        };
        for budget in [0, 1, 20_000, 200_000] {
            for bot in [&mut MinimaxBot as &mut dyn Bot<TicTacToe>, &mut RandomBot] {
                let command = bot
                    .decide(&game, slot, &mut rng, &mut Budget::new(budget))
                    .unwrap_or_else(|| panic!("{bot:?} gave up, budget {budget}"));
                let mut copy = game.clone();
                assert!(copy.apply_command(slot, &command).is_ok(), "{command:?}");
            }
        }
    }
}

#[test]
fn minimax_takes_a_win_before_blocking() {
    // win_count 4, X to move: both sides have three in a row.
    let game = position(10, 4, &[(4, 7), (5, 7), (6, 7)], &[(4, 9), (5, 9), (6, 9)]);
    let point = decide(&mut MinimaxBot, &game, 0);
    assert!(
        [Point::new(3, 7), Point::new(7, 7)].contains(&point),
        "{point:?}"
    );
}

#[test]
fn minimax_blocks_an_immediate_loss() {
    // O (to move) faces X's closed four on row 0: only (4,0) saves the game.
    let game = position(
        10,
        5,
        &[(0, 0), (1, 0), (2, 0), (3, 0)],
        &[(5, 5), (9, 9), (0, 9)],
    );
    assert_eq!(decide(&mut MinimaxBot, &game, 0), Point::new(4, 0));
}

#[test]
fn minimax_makes_an_open_four() {
    let game = position(
        15,
        5,
        &[(5, 7), (6, 7), (7, 7)],
        &[(0, 0), (14, 0), (0, 14)],
    );
    let point = decide(&mut MinimaxBot, &game, 0);
    assert!(
        [Point::new(4, 7), Point::new(8, 7)].contains(&point),
        "{point:?}"
    );
}

#[test]
fn minimax_stops_an_open_three_from_becoming_open_four() {
    // O to move; X has an open three on row 7.
    let game = position(15, 5, &[(5, 7), (6, 7), (7, 7)], &[(0, 0), (14, 0)]);
    let point = decide(&mut MinimaxBot, &game, 0);
    assert!(
        [Point::new(4, 7), Point::new(8, 7)].contains(&point),
        "{point:?}"
    );
}

#[test]
fn minimax_prevents_a_double_threat() {
    // O to move. X on (7,7) would make two closed fours with different winning cells.
    let xs = [(8, 7), (9, 7), (10, 7), (7, 6), (7, 5), (7, 4)];
    let os = [(11, 7), (7, 3), (0, 0), (14, 0), (0, 14)];
    let game = position(15, 5, &xs, &os);
    assert_eq!(decide(&mut MinimaxBot, &game, 0), Point::new(7, 7));
}

#[test]
fn minimax_finds_a_fork_through_search() {
    // X to move. Row 7 and column 7 each hold three X closed by O on one end; (7,7) makes two
    // fours with different winning cells, which O cannot both block. No one-ply rule sees it.
    let xs = [(4, 7), (5, 7), (6, 7), (7, 4), (7, 5), (7, 6)];
    let os = [(3, 7), (7, 3), (0, 0), (14, 0), (0, 14), (14, 14)];
    let game = position(15, 5, &xs, &os);
    assert_eq!(
        decide(&mut MinimaxBot, &game, MINIMAX_BOT_BUDGET),
        Point::new(7, 7)
    );
}

/// T4: legacy scored wins as ±1000 while heuristics reached far beyond, so a bad-looking but
/// safe position could lose to a "good" one that loses outright.
#[test]
fn terminal_scores_dominate_every_heuristic() {
    assert!(Score::win_for(Side::Opponent, 1) < Score::heuristic(i64::MIN));
    assert!(Score::win_for(Side::Bot, 400) > Score::heuristic(i64::MAX));
    assert!(Score::win_for(Side::Bot, 3) > Score::win_for(Side::Bot, 5));
    assert!(Score::win_for(Side::Opponent, 5) > Score::win_for(Side::Opponent, 3));
    assert!(Score::heuristic(i64::MAX).value() < Score::win_for(Side::Bot, 400).value());
}

#[test]
fn search_blocks_a_loss_even_when_attacking_looks_better() {
    // X to move: an open four at (8,7) or (4,7) scores best heuristically, but O then completes
    // row 0 at (4,0). The search alone (no one-ply rules) must block.
    let xs = [(5, 7), (6, 7), (7, 7), (12, 12)];
    let os = [(0, 0), (1, 0), (2, 0), (3, 0)];
    let game = position(15, 5, &xs, &os);
    let mut board = SearchBoard::new(&game, Mark::X);
    let mut root: Vec<Candidate> = Vec::new();
    board.collect_candidates(Neighbourhood::WithinTwo, &mut root);
    for candidate in &mut root {
        candidate.probe = board.probe(candidate.point);
    }
    let heuristic_favourite = root
        .iter()
        .max_by_key(|candidate| candidate.probe.bot.gain)
        .unwrap()
        .point;
    assert_ne!(heuristic_favourite, Point::new(4, 0));
    let mut budget = Budget::new(2_000_000);
    let chosen = Search::new(&mut board, &mut budget)
        .best_move(&root)
        .unwrap();
    assert_eq!(chosen.point, Point::new(4, 0));
}

/// T5: legacy `1 << (count * 2)` and `* 16` overflowed i32 on long lines (debug builds panic).
#[test]
fn long_lines_and_full_boards_do_not_overflow() {
    let mut game = new_game(20, 20, 20);
    for x in 0..19 {
        place(&mut game, x, 0);
        place(&mut game, x, 1);
    }
    assert_eq!(game.end(), None);
    assert_eq!(
        decide(&mut MinimaxBot, &game, TEST_BUDGET),
        Point::new(19, 0)
    );

    let mut rng = CoreRng::new(5);
    let crowded = random_position(20, 20, 390, &mut rng);
    if crowded.current_player().is_some() {
        decide(&mut MinimaxBot, &crowded, TEST_BUDGET);
    }
}

/// T6/T7: same position and budget, same move, whatever the rng.
#[test]
fn minimax_is_deterministic() {
    let mut rng = CoreRng::new(3);
    for _ in 0..10 {
        let game = random_position(15, 5, 12, &mut rng);
        let Some(me) = game.current_player() else {
            continue;
        };
        let decisions: Vec<Option<Command>> = (0..3)
            .map(|seed| {
                MinimaxBot.decide(
                    &game,
                    me,
                    &mut CoreRng::new(seed),
                    &mut Budget::new(200_000),
                )
            })
            .collect();
        assert!(decisions.windows(2).all(|pair| pair[0] == pair[1]));
    }
}

#[test]
fn minimax_spends_at_most_its_budget() {
    let game = position(15, 5, &[(7, 7), (8, 8)], &[(7, 8)]);
    for cells in [0, 1, 1_000, 100_000] {
        let mut budget = Budget::new(cells);
        MinimaxBot.decide(&game, GUEST, &mut CoreRng::new(0), &mut budget);
        assert!(budget.remaining() <= cells);
    }
    let mut budget = Budget::new(TEST_BUDGET);
    MinimaxBot.decide(&game, GUEST, &mut CoreRng::new(0), &mut budget);
    assert!(
        budget.remaining() < TEST_BUDGET / 2,
        "the search uses its budget"
    );
}

/// The budget bounds the work actually done on every board: it used to be charged one node per
/// searched move while each node probed every candidate, so long lines cost far more per node
/// (20×20 / 20 took ~2 s per move).
#[test]
fn search_work_stays_within_the_budget_on_every_board() {
    let mut rng = CoreRng::new(17);
    for (size, win_count) in [(20, 20), (20, 10), (15, 5), (10, 10), (3, 3)] {
        let cells = u64::from(size * size);
        let probe_cells = 4 * (2 * u64::from(win_count) - 1);
        // Work between two charges: at most one probe and one move update per cell, plus three
        // candidate scans.
        let overshoot = cells * (probe_cells + 25 + 3);
        for moves in [1, size * size / 4, size * size / 2] {
            let game = random_position(size, win_count, moves as usize, &mut rng);
            let Some(me) = game.current_player() else {
                continue;
            };
            let mark = game.mark_of(me).unwrap();
            for budget in [0, 50_000, 400_000] {
                let mut board = SearchBoard::new(&game, mark);
                let mut root: Vec<Candidate> = Vec::new();
                board.collect_candidates(Neighbourhood::WithinTwo, &mut root);
                for candidate in &mut root {
                    candidate.probe = board.probe(candidate.point);
                }
                let root_work = board.work_done();
                Search::new(&mut board, &mut Budget::new(budget)).best_move(&root);
                assert!(
                    board.work_done() <= root_work.max(budget) + overshoot,
                    "{size}x{size}/{win_count} after {moves} moves, budget {budget}: {} cells",
                    board.work_done()
                );
            }
        }
    }
}

#[test]
fn minimax_never_loses_three_by_three() {
    for seed in 0..40 {
        for minimax_seat in [HOST, GUEST] {
            let mut game = new_game(3, 3, 3);
            let (mut minimax, mut random) = (MinimaxBot, RandomBot);
            let bots: [&mut dyn Bot<TicTacToe>; 2] = if minimax_seat == HOST {
                [&mut minimax, &mut random]
            } else {
                [&mut random, &mut minimax]
            };
            let end = play_out(&mut game, bots, [MINIMAX_BOT_BUDGET; 2], seed);
            let opponent = Slot::new(1 - minimax_seat.get());
            assert_ne!(end.winner(), Some(opponent), "seed {seed}");
        }
    }
    let mut game = new_game(3, 3, 3);
    let end = play_out(
        &mut game,
        [&mut MinimaxBot, &mut MinimaxBot],
        [MINIMAX_BOT_BUDGET; 2],
        0,
    );
    assert_eq!(end, GameEnd::Draw);
}

#[test]
fn minimax_beats_random_on_a_big_board() {
    for seed in 0..4 {
        for minimax_seat in [HOST, GUEST] {
            let mut game = new_game(15, 15, 5);
            let (mut minimax, mut random) = (MinimaxBot, RandomBot);
            let bots: [&mut dyn Bot<TicTacToe>; 2] = if minimax_seat == HOST {
                [&mut minimax, &mut random]
            } else {
                [&mut random, &mut minimax]
            };
            let end = play_out(&mut game, bots, [TEST_BUDGET; 2], seed);
            assert_eq!(end.winner(), Some(minimax_seat), "seed {seed}");
        }
    }
}
