//! Release-build timings for the default minimax budget (successor of the legacy criterion
//! bench): `cargo test -p mg-tictactoe --release -- --ignored --nocapture timing`.

#![allow(clippy::print_stdout, reason = "reports timings to the developer")]

use std::time::{Duration, Instant};

use mg_core::{Bot, Budget, CoreRng, Game};

use super::{decide, new_game, place, position};
use crate::{MINIMAX_BOT_BUDGET, MinimaxBot, RandomBot, TicTacToe};

const LIMIT: Duration = Duration::from_millis(200);

fn timed_move(game: &TicTacToe) -> Duration {
    let started = Instant::now();
    decide(&mut MinimaxBot, game, MINIMAX_BOT_BUDGET);
    started.elapsed()
}

/// Minimax (X) against the random bot; returns the slowest and the mean minimax move.
fn against_random(size: u32, seed: u64) -> (Duration, Duration) {
    let mut game = new_game(size, size, 5);
    let mut rng = CoreRng::new(seed);
    let mut times = Vec::new();
    while game.end().is_none() {
        let slot = game.current_player().unwrap();
        if slot.get() == 0 {
            times.push(timed_move(&game));
            let point = decide(&mut MinimaxBot, &game, MINIMAX_BOT_BUDGET);
            place(&mut game, point.x, point.y);
        } else {
            let command = RandomBot
                .decide(&game, slot, &mut rng, &mut Budget::new(1))
                .unwrap();
            game.apply_command(slot, &command).unwrap();
        }
    }
    let total: Duration = times.iter().sum();
    let slowest = times.iter().copied().max().unwrap_or_default();
    (slowest, total / times.len().max(1) as u32)
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

    let mut slowest = Duration::ZERO;
    for size in [15, 20] {
        for seed in 0..4 {
            let (max, mean) = against_random(size, seed);
            println!("{size}x{size} vs random, seed {seed}: slowest {max:?}, mean {mean:?}");
            slowest = slowest.max(max);
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
