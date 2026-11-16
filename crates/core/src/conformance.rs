//! Generic test-suite every game must pass (enable feature `conformance` in dev-dependencies).
//!
//! A game crate implements [`Conformance`] for its game type and calls [`run_all`] from a test.
//! The suite checks, for every sample setup and many seeds:
//! * invalid settings are rejected by `Game::new`;
//! * random (mostly illegal) command streams never panic;
//! * rejected commands leave the state untouched;
//! * every frame the [`Authority`] seals replays to the identical state on a [`Match`];
//! * the compact log replays to the same final state (replays/catch-up);
//! * game-specific invariants hold after every frame;
//! * bot-only tables finish within the frame limit, and finished games reject commands.

use crate::authority::{Authority, BotSeat, expand_log};
use crate::game::{Awaiting, Game, PlayerKind, PlayerSpec, Slot};
use crate::matcher::Match;
use crate::rng::CoreRng;

/// Hooks a game provides to the suite.
pub trait Conformance: Game {
    /// Valid setups to exercise (cover edge sizes, all modes, bot seats).
    fn sample_setups() -> Vec<(Self::Settings, Vec<PlayerSpec>)>;

    /// Setups `Game::new` must reject (out-of-range values, wrong seat counts, huge sizes).
    fn invalid_setups() -> Vec<(Self::Settings, Vec<PlayerSpec>)>;

    /// An arbitrary command for fuzzing; may be illegal. Bias it towards plausible moves so games
    /// actually progress.
    fn arbitrary_command(game: &Self, slot: Slot, rng: &mut CoreRng) -> Self::Command;

    /// Bots for the bot seats of `players`.
    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Self>> {
        let _ = players;
        Vec::new()
    }

    /// Game-specific invariants, checked after every frame.
    fn check_invariants(&self) -> Result<(), String> {
        Ok(())
    }

    /// Frame limit per simulated match.
    fn max_frames() -> u64 {
        3_000
    }
}

/// Runs the whole suite with `seeds` seeds per setup. Panics with a descriptive message on failure.
pub fn run_all<G: Conformance>(seeds: u64) {
    rejects_invalid_setups::<G>();
    for (index, (settings, players)) in G::sample_setups().into_iter().enumerate() {
        for seed in 0..seeds {
            let mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ index as u64;
            simulate::<G>(&settings, &players, mixed)
                .unwrap_or_else(|e| panic!("{}: setup #{index}, seed {mixed}: {e}", G::INFO.id));
        }
    }
}

/// Every invalid setup must be rejected.
pub fn rejects_invalid_setups<G: Conformance>() {
    for (index, (settings, players)) in G::invalid_setups().into_iter().enumerate() {
        if G::new(&settings, &players).is_ok() {
            panic!(
                "{}: invalid setup #{index} was accepted: {settings:?} / {players:?}",
                G::INFO.id
            );
        }
    }
}

/// Final state hash after a scripted random match; use for golden tests that must be identical
/// on every platform (native, wasm32).
pub fn scripted_hash<G: Conformance>(
    settings: &G::Settings,
    players: &[PlayerSpec],
    seed: u64,
) -> u64 {
    match simulate::<G>(settings, players, seed) {
        Ok(hash) => hash,
        Err(e) => panic!("{}: scripted match failed: {e}", G::INFO.id),
    }
}

/// Simulates one match driven by random inputs; returns the final state hash.
pub fn simulate<G: Conformance>(
    settings: &G::Settings,
    players: &[PlayerSpec],
    seed: u64,
) -> Result<u64, String> {
    let mut authority = Authority::<G>::new(settings, players, seed, G::bots_for(players))
        .map_err(|e| format!("sample setup rejected: {e}"))?;
    let mut mirror = Match::<G>::new(settings, players).map_err(|e| format!("mirror: {e}"))?;
    let mut fuzz = CoreRng::derive(seed, 0xF022);
    let humans: Vec<Slot> = players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == PlayerKind::Human)
        .filter_map(|(i, _)| Slot::try_from(i).ok())
        .collect();
    let realtime = matches!(
        authority.game().clock(),
        crate::game::Clock::Realtime { .. }
    );

    // Setup frame (initial randomness).
    let frame = authority.seal();
    mirror
        .apply_frame(&frame)
        .map_err(|e| format!("setup frame: {e}"))?;
    authority.game().check_invariants()?;

    for _ in 0..G::max_frames() {
        if authority.game().status().is_finished() {
            break;
        }
        if realtime {
            for &slot in &humans {
                if fuzz.chance(1, 3) {
                    try_submit(&mut authority, slot, &mut fuzz)?;
                }
            }
            if fuzz.chance(1, 400) && !humans.is_empty() {
                let slot = humans[fuzz.below(humans.len() as u32) as usize];
                authority.drop_player(slot);
            }
            authority.run_bots();
        } else {
            match authority.game().awaiting() {
                Awaiting::Turn(slot) if humans.contains(&slot) => {
                    let mut accepted = false;
                    for _ in 0..64 {
                        if try_submit(&mut authority, slot, &mut fuzz)? {
                            accepted = true;
                            break;
                        }
                    }
                    if !accepted {
                        authority.drop_player(slot);
                    }
                }
                Awaiting::Simultaneous => {
                    for &slot in &humans {
                        try_submit(&mut authority, slot, &mut fuzz)?;
                    }
                    authority.run_bots();
                }
                _ => authority.run_bots(),
            }
            if !authority.has_pending() {
                // Nothing happened this turn (e.g. no legal move found): force progress.
                if let Awaiting::Turn(slot) = authority.game().awaiting() {
                    authority.drop_player(slot);
                }
            }
        }
        let frame = authority.seal();
        mirror
            .apply_frame(&frame)
            .map_err(|e| format!("tick {}: {e}", frame.tick))?;
        let (a, m) = (authority.game().state_hash(), mirror.game().state_hash());
        if a != m {
            return Err(format!(
                "tick {}: authority {a:016x} != mirror {m:016x}",
                frame.tick
            ));
        }
        authority
            .game()
            .check_invariants()
            .map_err(|e| format!("tick {}: invariant: {e}", frame.tick))?;
    }

    let all_bots = players.iter().all(|p| p.kind == PlayerKind::Bot);
    if all_bots && !authority.game().status().is_finished() && !realtime {
        return Err(format!(
            "bot-only turn-based match did not finish in {} frames",
            G::max_frames()
        ));
    }
    if authority.game().status().is_finished() {
        for &slot in &humans {
            let cmd = G::arbitrary_command(authority.game(), slot, &mut fuzz);
            if authority.submit(slot, cmd).is_ok() {
                return Err("finished game accepted a command".into());
            }
        }
    }

    // Replay the compact log from scratch.
    let mut replay = Match::<G>::new(settings, players).map_err(|e| format!("replay: {e}"))?;
    let frames = expand_log(authority.log(), authority.next_tick());
    replay
        .apply_all(&frames)
        .map_err(|e| format!("replay: {e}"))?;
    let final_hash = authority.game().state_hash();
    if replay.game().state_hash() != final_hash {
        return Err("replayed log diverged from the authority".into());
    }
    Ok(final_hash)
}

/// Submits one arbitrary command; checks that rejection leaves state untouched.
fn try_submit<G: Conformance>(
    authority: &mut Authority<G>,
    slot: Slot,
    fuzz: &mut CoreRng,
) -> Result<bool, String> {
    let cmd = G::arbitrary_command(authority.game(), slot, fuzz);
    let before = authority.game().state_hash();
    match authority.submit(slot, cmd.clone()) {
        Ok(()) => Ok(true),
        Err(_) => {
            if authority.game().state_hash() != before {
                return Err(format!("rejected command {cmd:?} changed the state"));
            }
            Ok(false)
        }
    }
}
