use std::sync::atomic::{AtomicU64, Ordering};

use mg_core::{Slot, Status};
use mg_registry::{CreateError, GameKind, SeatSpec, create_authority};

use crate::arena_config::ArenaConfig;
use crate::arena_error::ArenaError;
use crate::arena_report::ArenaReport;
use crate::game_seed::game_seed;
use crate::played_game::PlayedGame;
use crate::replay_writer::write_replay;
use crate::seat_tally::{SeatResult, SeatTally, seat_result};

pub fn run_arena(config: &ArenaConfig) -> Result<ArenaReport, ArenaError> {
    let game = GameKind::of_settings(&config.settings).map_err(CreateError::from)?;
    let seats: Vec<SeatSpec> = config
        .bots
        .iter()
        .map(|bot| SeatSpec::Bot(bot.kind))
        .collect();
    // Fails fast on a bad setup instead of once per worker.
    create_authority(&config.settings, &seats, config.seed)?;
    if let Some(dir) = &config.replays {
        std::fs::create_dir_all(dir).map_err(|source| ArenaError::WriteReplay {
            path: dir.clone(),
            source,
        })?;
    }

    let next_index = AtomicU64::new(0);
    let workers = config
        .threads
        .get()
        .min(usize::try_from(config.games).unwrap_or(usize::MAX))
        .max(1);
    let per_worker: Vec<Result<Vec<PlayedGame>, ArenaError>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| scope.spawn(|| play_games(config, &seats, &next_index)))
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });

    let mut report = ArenaReport {
        game,
        games: config.games,
        seed: config.seed,
        seats: Slot::all(config.bots.len())
            .zip(&config.bots)
            .map(|(slot, bot)| SeatTally::new(slot, bot.clone()))
            .collect(),
        total_ticks: 0,
        replays_written: 0,
    };
    for played in per_worker {
        for game in played? {
            report.total_ticks += game.length.get();
            report.replays_written += u32::from(game.replay_written);
            for seat in &mut report.seats {
                seat.record(game.outcome.as_ref());
            }
        }
    }
    Ok(report)
}

/// Takes game indices until none are left. Which worker plays a game does not matter:
/// everything about it follows from its index.
fn play_games(
    config: &ArenaConfig,
    seats: &[SeatSpec],
    next_index: &AtomicU64,
) -> Result<Vec<PlayedGame>, ArenaError> {
    let mut played = Vec::new();
    loop {
        let index = next_index.fetch_add(1, Ordering::Relaxed);
        let Some(index) = u32::try_from(index)
            .ok()
            .filter(|index| *index < config.games)
        else {
            return Ok(played);
        };
        played.push(play_game(config, seats, index)?);
    }
}

fn play_game(
    config: &ArenaConfig,
    seats: &[SeatSpec],
    index: u32,
) -> Result<PlayedGame, ArenaError> {
    let mut authority = create_authority(&config.settings, seats, game_seed(config.seed, index))?;
    while !authority.status().is_finished() && authority.next_tick() < config.max_ticks {
        authority.run_bots();
        authority.seal_frame();
    }
    let outcome = match authority.status() {
        Status::Finished(outcome) => Some(outcome),
        Status::InProgress => None,
    };
    let mut replay_written = false;
    if let (Some(dir), Some(outcome)) = (&config.replays, &outcome)
        && seat_result(outcome, Slot::new(0)) == SeatResult::Lost
    {
        write_replay(dir, config, index, authority.as_ref(), outcome)?;
        replay_written = true;
    }
    Ok(PlayedGame {
        length: authority.next_tick(),
        outcome,
        replay_written,
    })
}
