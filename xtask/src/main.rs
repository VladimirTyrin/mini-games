//! `cargo xtask new-game <snake_case_id> [--title "Display Name"]`
//!
//! Scaffolds a working game everywhere it has to exist — game crate, proto messages, wire
//! mapping, registry entry, web plugin — so a new game compiles, passes the conformance suite and
//! is playable before any real rule is written. See docs/adding-a-game.md.

// A command-line tool: talking to the terminal is its job.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod game_names;
mod scaffold;

use std::path::PathBuf;
use std::process::ExitCode;

use game_names::GameNames;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("new-game") => new_game(&args[1..]),
        _ => {
            Err("usage: cargo xtask new-game <snake_case_id> [--title \"Display Name\"]".to_owned())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn new_game(args: &[String]) -> Result<(), String> {
    let id = args.first().ok_or("missing game id")?;
    let title = match args.get(1).map(String::as_str) {
        Some("--title") => Some(args.get(2).ok_or("--title needs a value")?.clone()),
        Some(other) => return Err(format!("unexpected argument `{other}`")),
        None => None,
    };
    let names = GameNames::new(id, title)?;
    let root = repo_root()?;
    scaffold::create_game(&root, &names)?;
    println!("Created game `{}`. Next:", names.snake);
    println!(
        "  1. cargo test -p mg-{}            # conformance passes out of the box",
        names.kebab
    );
    println!(
        "  2. cd client && npm run dev   # Home → {} plays locally",
        names.title
    );
    println!("  3. Replace the sample rules; follow docs/adding-a-game.md");
    Ok(())
}

fn repo_root() -> Result<PathBuf, String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "cannot locate the repository root".to_owned())
}
