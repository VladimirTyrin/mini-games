//! A network bot playing one of the core's built-in bots against a server.
//!
//! ```text
//! MG_BOT_KEY=secret cargo run -p mg-bot-sdk --example network_bot -- \
//!     --url ws://127.0.0.1:5000/ws --game tictactoe --bot minimax --opponents random
//! ```

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use mg_arena::{ArenaBot, game_by_id, settings_from_json};
use mg_bot_sdk::{BotConfig, BotSession, BuiltinBot, LobbySetup, MatchSummary, SdkError};
use mg_protocol::{WireBots, proto};
use mg_registry::GameKind;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long, default_value = "ws://127.0.0.1:5000/ws")]
    url: String,
    /// Bot key issued by the server operator.
    #[arg(long, env = "MG_BOT_KEY", hide_env_values = true)]
    key: String,
    #[arg(long, default_value = "tictactoe")]
    game: String,
    /// Built-in bot this client plays with.
    #[arg(long, default_value = "minimax")]
    bot: String,
    /// Join this lobby instead of hosting one.
    #[arg(long)]
    join: Option<String>,
    /// Built-in server bots seated against us when hosting, e.g. `random`.
    #[arg(long, value_delimiter = ',')]
    opponents: Vec<String>,
    /// Seats of a hosted lobby (default: us plus the opponents); extra seats wait for other
    /// players to join.
    #[arg(long)]
    seats: Option<u32>,
    /// Game settings as protobuf JSON of `GameSettings` (default: the game's own).
    #[arg(long)]
    settings_json: Option<String>,
    #[arg(long, default_value_t = 1)]
    matches: u32,
    #[arg(long, default_value_t = 1)]
    seed: u64,
}

#[derive(Debug, thiserror::Error)]
enum ExampleError {
    #[error(transparent)]
    Arena(#[from] mg_arena::ArenaError),
    #[error(transparent)]
    Sdk(#[from] SdkError),
    #[error(transparent)]
    Wire(#[from] mg_protocol::WireError),
    #[error("{0} has no built-in bots")]
    NoBots(&'static str),
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ignored = writeln!(std::io::stderr(), "network_bot: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), ExampleError> {
    let game = game_by_id(&cli.game)?;
    match game {
        GameKind::TicTacToe => play::<mg_tictactoe::TicTacToe>(&cli, game).await,
        GameKind::Snake => play::<mg_snake::Snake>(&cli, game).await,
        other => Err(ExampleError::NoBots(other.info().id)),
    }
}

async fn play<G: WireBots>(cli: &Cli, game: GameKind) -> Result<(), ExampleError> {
    let mine = ArenaBot::find(game, &cli.bot)?;
    let opponents = cli
        .opponents
        .iter()
        .map(|name| ArenaBot::find(game, name).map(|bot| bot.kind))
        .collect::<Result<Vec<_>, _>>()?;
    let settings = match &cli.settings_json {
        Some(json) => settings_from_json(game, json)?,
        None => mg_registry::default_settings(game),
    };
    let setup = LobbySetup {
        name: format!("{} vs bots", mine.name),
        settings,
        max_seats: cli
            .seats
            .unwrap_or(1 + u32::try_from(opponents.len()).unwrap_or(u32::MAX)),
        built_in_bots: opponents,
    };

    let mut session = BotSession::connect(BotConfig::new(&cli.url, &cli.key)).await?;
    for round in 0..cli.matches {
        let header = match &cli.join {
            Some(lobby_id) if round == 0 => session.join_match(lobby_id).await?,
            Some(_) => session.wait_for_match().await?,
            None => session.host_match(&setup).await?,
        };
        let mut strategy = BuiltinBot::<G>::from_wire(&mine.kind, cli.seed + u64::from(round))?;
        let summary = session.play_match(header, &mut strategy).await?;
        report(round, &summary);
        if cli.join.is_none() {
            session.leave_lobby().await?;
        } else if round + 1 < cli.matches {
            session
                .request(proto::client_message::Message::VotePlayAgain(
                    proto::VotePlayAgain {},
                ))
                .await?;
        }
    }
    Ok(())
}

fn report(round: u32, summary: &MatchSummary) {
    let outcome = match &summary.result {
        None => "aborted",
        Some(_) if summary.won() => "won",
        Some(result) if result.kind == proto::OutcomeKind::Draw as i32 => "draw",
        Some(_) => "lost",
    };
    let _ignored = writeln!(
        std::io::stdout(),
        "match {} ({}): {outcome} after {} ticks, seat {:?}, reconnects {}",
        round + 1,
        summary.match_id,
        summary.final_tick,
        summary.my_slot,
        summary.reconnects
    );
}
