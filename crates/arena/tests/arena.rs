use std::num::NonZeroUsize;
use std::path::PathBuf;

use mg_arena::{ArenaBot, ArenaConfig, ArenaError, game_by_id, run_arena, settings_from_json};
use mg_core::{Status, Tick};
use mg_registry::{GameKind, load_replay_file};

const SMALL_TICTACTOE: &str = r#"{"tictactoe":{"fieldWidth":3,"fieldHeight":3,"winCount":3,"firstPlayer":"TIC_TAC_TOE_FIRST_PLAYER_MODE_RANDOM"}}"#;

fn tictactoe_config(bots: &[&str], games: u32, seed: u64, threads: usize) -> ArenaConfig {
    let game = game_by_id("tictactoe").unwrap();
    ArenaConfig {
        settings: settings_from_json(game, SMALL_TICTACTOE).unwrap(),
        bots: bots
            .iter()
            .map(|name| ArenaBot::find(game, name).unwrap())
            .collect(),
        games,
        seed,
        threads: NonZeroUsize::new(threads).unwrap(),
        max_ticks: Tick::new(1_000),
        replays: None,
    }
}

#[test]
fn results_depend_on_the_seed_not_on_the_thread_count() {
    let single = run_arena(&tictactoe_config(&["random", "random"], 60, 7, 1)).unwrap();
    let parallel = run_arena(&tictactoe_config(&["random", "random"], 60, 7, 4)).unwrap();
    assert_eq!(single, parallel);
    let total = single.seats[0].wins + single.seats[0].draws + single.seats[0].losses;
    assert_eq!(total, 60);
}

#[test]
fn minimax_beats_random() {
    let report = run_arena(&tictactoe_config(&["minimax", "random"], 40, 42, 4)).unwrap();
    let minimax = &report.seats[0];
    let random = &report.seats[1];
    assert_eq!(minimax.losses, 0, "{report}");
    assert!(minimax.wins > random.wins, "{report}");
    assert_eq!(minimax.unfinished, 0);
}

#[test]
fn replays_of_the_first_bots_losses_replay_to_the_recorded_result() {
    let dir = scratch_dir("arena-replays");
    let mut config = tictactoe_config(&["random", "minimax"], 12, 3, 2);
    config.replays = Some(dir.clone());
    let report = run_arena(&config).unwrap();
    assert!(report.seats[0].losses > 0, "{report}");
    assert_eq!(report.replays_written, report.seats[0].losses);

    let mut files = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        let mut replay = load_replay_file(&bytes).unwrap();
        let end = replay.log.end();
        replay.mirror.apply_compact_log(&replay.log, end).unwrap();
        let Status::Finished(outcome) = replay.mirror.status() else {
            panic!("replay did not reach the end");
        };
        assert_eq!(outcome.winners, vec![mg_core::Slot::new(1)]);
        files += 1;
    }
    assert_eq!(files, report.replays_written);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_names_and_foreign_settings_are_explained() {
    let tictactoe = game_by_id("tictactoe").unwrap();
    assert!(matches!(
        ArenaBot::find(tictactoe, "alphazero"),
        Err(ArenaError::UnknownBot { known, .. }) if known == ["minimax", "random"]
    ));
    assert!(matches!(
        game_by_id("chess"),
        Err(ArenaError::UnknownGame { .. })
    ));
    let snake = game_by_id("snake").unwrap();
    assert_eq!(snake, GameKind::Snake);
    assert!(matches!(
        settings_from_json(snake, SMALL_TICTACTOE),
        Err(ArenaError::SettingsForAnotherGame { .. })
    ));
}

/// A fresh directory under the target dir, so parallel test runs do not share files.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}
