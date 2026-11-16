use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use mg_arena::{
    ArenaBot, ArenaConfig, ArenaError, ArenaReport, game_by_id, run_arena, settings_from_json,
};
use mg_core::Tick;

/// Plays built-in bots against each other locally and prints per-seat results.
#[derive(Debug, Parser)]
#[command(name = "mg-arena", version)]
struct Cli {
    /// Game id: snake, tictactoe, ...
    #[arg(long)]
    game: String,
    /// One bot per seat in slot order, e.g. `minimax,random`.
    #[arg(long, value_delimiter = ',', required = true)]
    bots: Vec<String>,
    #[arg(long, default_value_t = 100)]
    games: u32,
    /// Base seed; every game derives its own seed from it.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Worker threads (default: available cores). Results do not depend on it.
    #[arg(long)]
    threads: Option<NonZeroUsize>,
    /// Game settings as protobuf JSON of `GameSettings`, e.g.
    /// `{"tictactoe":{"fieldWidth":3,"fieldHeight":3,"winCount":3,"firstPlayer":"TIC_TAC_TOE_FIRST_PLAYER_MODE_RANDOM"}}`.
    /// Default: the game's own.
    #[arg(long)]
    settings_json: Option<String>,
    /// Games still running after this many frames count as unfinished.
    #[arg(long, default_value_t = 100_000)]
    max_ticks: u64,
    /// Directory for replays (`.mgreplay`) of the games the first bot lost.
    #[arg(long)]
    replays: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(report) => {
            // A closed stdout (output piped into `head`) is not an arena failure.
            let _ignored = write!(std::io::stdout().lock(), "{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ignored = writeln!(std::io::stderr().lock(), "mg-arena: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ArenaReport, ArenaError> {
    let game = game_by_id(&cli.game)?;
    let settings = match &cli.settings_json {
        Some(json) => settings_from_json(game, json)?,
        None => mg_registry::default_settings(game),
    };
    let bots = cli
        .bots
        .iter()
        .map(|name| ArenaBot::find(game, name.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    let threads = cli
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN));
    run_arena(&ArenaConfig {
        settings,
        bots,
        games: cli.games,
        seed: cli.seed,
        threads,
        max_ticks: Tick::new(cli.max_ticks),
        replays: cli.replays,
    })
}
