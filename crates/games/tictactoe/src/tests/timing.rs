//! Release-build timings for the default minimax budget (successor of the legacy criterion
//! bench): `cargo test -p mg-tictactoe --release -- --ignored --nocapture timing`.

#![allow(clippy::print_stdout, reason = "reports timings to the developer")]

use std::time::{Duration, Instant};

use mg_core::{Bot, Budget, CoreRng};

use super::{decide, new_game, place, position};
use crate::{Command, MINIMAX_BOT_BUDGET, MinimaxBot, RandomBot, TicTacToe};

const LIMIT: Duration = Duration::from_millis(150);

fn timed_move(game: &TicTacToe) -> Duration {
    let started = Instant::now();
    decide(&mut MinimaxBot, game, MINIMAX_BOT_BUDGET);
    started.elapsed()
}

/// Minimax (X) against the random bot on a square board.
fn against_random(size: u32, win_count: u32, seed: u64) -> MoveTimes {
    let mut game = new_game(size, size, win_count);
    let mut rng = CoreRng::new(seed);
    let mut times = Vec::new();
    while game.end().is_none() {
        let slot = game.current_player().unwrap();
        let command = if slot.get() == 0 {
            let started = Instant::now();
            let command =
                MinimaxBot.decide(&game, slot, &mut rng, &mut Budget::new(MINIMAX_BOT_BUDGET));
            times.push(started.elapsed());
            command
        } else {
            RandomBot.decide(&game, slot, &mut rng, &mut Budget::new(1))
        };
        let Some(Command::Place(point)) = command else {
            panic!("bot returned no move");
        };
        place(&mut game, point.x, point.y);
    }
    let total: Duration = times.iter().sum();
    MoveTimes {
        slowest: times.iter().copied().max().unwrap_or_default(),
        mean: total / times.len().max(1) as u32,
    }
}

struct MoveTimes {
    slowest: Duration,
    mean: Duration,
}

#[test]
#[ignore = "timing; run in release"]
fn timing_minimax() {
    let empty = timed_move(&new_game(15, 15, 5));
    println!("15x15 empty board: {empty:?}");

    let xs = [(7, 7), (7, 8), (6, 6), (5, 5), (6, 8)];
    let os = [(8, 7), (8, 8), (9, 9), (10, 10), (8, 6)];
    let early = timed_move(&position(15, 5, &xs, &os));
    println!("15x15 after 10 moves (legacy bench opening): {early:?}");

    // Long lines make probes expensive: 20×20 / 10..=20 are the worst cases per move.
    let runs = [
        (15, 5, 4),
        (20, 5, 2),
        (20, 10, 2),
        (20, 15, 1),
        (20, 20, 1),
        (10, 10, 1),
    ];
    let mut slowest = Duration::ZERO;
    for (size, win_count, seeds) in runs {
        for seed in 0..seeds {
            let times = against_random(size, win_count, seed);
            println!(
                "{size}x{size}/{win_count} vs random, seed {seed}: slowest {:?}, mean {:?}",
                times.slowest, times.mean
            );
            slowest = slowest.max(times.slowest);
        }
    }

    let mut game = new_game(15, 15, 5);
    for _ in 0..60 {
        if game.end().is_some() {
            break;
        }
        let point = decide(&mut MinimaxBot, &game, MINIMAX_BOT_BUDGET);
        place(&mut game, point.x, point.y);
    }
    println!("15x15 minimax vs itself: {:?}", game.end());

    assert!(empty.max(early).max(slowest) < LIMIT);
}
