// A test harness: panicking with a descriptive message is how it fails a game's test.
#![allow(clippy::panic)]

use super::contract::Conformance;
use super::setup::Setup;
use crate::authority::Authority;
use crate::awaiting::Awaiting;
use crate::compact_log::CompactLog;
use crate::game_settings::GameSettings;
use crate::matcher::Match;
use crate::player_spec::PlayerSpec;
use crate::rng::CoreRng;
use crate::slot::Slot;

/// Simulates every sample setup with `seeds` seeds and checks: no panics on random input,
/// rejections leave state untouched, authority frames replay identically on a `Match`, the
/// compact log replays to the final state, invariants hold, bot-only turn-based tables finish,
/// finished games reject commands. Panics with a descriptive message.
pub fn run_all<G: Conformance>(seeds: u64) {
    rejects_invalid_setups::<G>();
    for (index, setup) in G::sample_setups().iter().enumerate() {
        for seed in 0..seeds {
            let seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ index as u64;
            if let Err(error) = simulate(setup, seed) {
                panic!(
                    "{}: setup #{index} {setup:?}, seed {seed}: {error}",
                    G::INFO.id
                );
            }
        }
    }
}

pub fn rejects_invalid_setups<G: Conformance>() {
    for (index, setup) in G::invalid_setups().iter().enumerate() {
        let accepted = G::Settings::parse(setup.settings.clone())
            .is_ok_and(|settings| G::new(&settings, &setup.players).is_ok());
        if accepted {
            panic!(
                "{}: invalid setup #{index} was accepted: {setup:?}",
                G::INFO.id
            );
        }
    }
}

/// Final state hash of a seeded random match. Pin it in a test to catch accidental rule changes
/// and to compare native with wasm32.
pub fn scripted_hash<G: Conformance>(setup: &Setup<G>, seed: u64) -> u64 {
    match simulate(setup, seed) {
        Ok(hash) => hash,
        Err(error) => panic!("{}: scripted match failed: {error}", G::INFO.id),
    }
}

/// Returns the final state hash.
pub fn simulate<G: Conformance>(setup: &Setup<G>, seed: u64) -> Result<u64, String> {
    let players: &[PlayerSpec] = &setup.players;
    let settings = G::Settings::parse(setup.settings.clone())
        .map_err(|e| format!("sample settings rejected: {e}"))?;
    if settings.to_raw() != setup.settings {
        return Err(format!(
            "settings do not round-trip: {:?} became {:?}",
            setup.settings,
            settings.to_raw()
        ));
    }
    let mut authority = Authority::<G>::new(&settings, players, seed, G::bots_for(players))
        .map_err(|e| format!("sample setup rejected: {e}"))?;
    let mut mirror = Match::<G>::new(&settings, players).map_err(|e| format!("mirror: {e}"))?;
    let mut fuzz = CoreRng::derive(seed, 0xF022);
    let humans: Vec<Slot> = Slot::all(players.len())
        .zip(players)
        .filter(|(_, player)| player.is_human())
        .map(|(slot, _)| slot)
        .collect();
    let realtime = authority.game().clock().is_realtime();

    let setup_frame = authority.seal_frame();
    mirror
        .apply_frame(&setup_frame)
        .map_err(|e| format!("setup frame: {e}"))?;
    authority.game().check_invariants()?;

    for _ in 0..G::max_frames() {
        if authority.game().status().is_finished() {
            break;
        }
        settle_human_secrets(&mut authority, &humans, &mut fuzz)?;
        if realtime {
            for &slot in &humans {
                if fuzz.chance(1, 3) {
                    submit_arbitrary(&mut authority, slot, &mut fuzz)?;
                }
            }
            if fuzz.chance(1, 400)
                && let Some(&slot) = fuzz.pick(&humans)
            {
                authority.drop_player(slot);
            }
            authority.run_bots();
        } else {
            drive_turn(&mut authority, &humans, &mut fuzz)?;
        }
        let frame = authority.seal_frame();
        mirror
            .apply_frame(&frame)
            .map_err(|e| format!("tick {}: {e}", frame.tick))?;
        let (expected, actual) = (authority.game().state_hash(), mirror.game().state_hash());
        if expected != actual {
            return Err(format!(
                "tick {}: authority {expected:016x} != mirror {actual:016x}",
                frame.tick
            ));
        }
        authority
            .game()
            .check_invariants()
            .map_err(|e| format!("tick {}: invariant: {e}", frame.tick))?;
    }

    if authority.rejected_bot_commands() > 0 {
        return Err(format!(
            "bots produced {} rejected commands",
            authority.rejected_bot_commands()
        ));
    }
    let bots_only = players.iter().all(|p| !p.is_human());
    if bots_only && !realtime && !authority.game().status().is_finished() {
        return Err(format!(
            "bot-only turn-based match did not finish in {} frames",
            G::max_frames()
        ));
    }
    if authority.game().status().is_finished() {
        for &slot in &humans {
            let command = G::arbitrary_command(authority.game(), slot, &mut fuzz);
            if authority.submit(slot, command).is_ok() {
                return Err("finished game accepted a command".into());
            }
        }
    }

    let mut replay = Match::<G>::new(&settings, players).map_err(|e| format!("replay: {e}"))?;
    let log = CompactLog::new(
        authority.compact_log().to_vec(),
        authority.next_tick(),
        authority.hash_policy(),
    )
    .map_err(|e| format!("compact log: {e}"))?;
    replay
        .apply_compact_log(&log, log.end())
        .map_err(|e| format!("replay: {e}"))?;
    let final_hash = authority.game().state_hash();
    if replay.game().state_hash() != final_hash {
        return Err("replayed log diverged from the authority".into());
    }
    Ok(final_hash)
}

/// People who owe a private command send an arbitrary one, which may be refused; whoever still
/// owes one then gets the game's default, as when a host's time limit runs out.
fn settle_human_secrets<G: Conformance>(
    authority: &mut Authority<G>,
    humans: &[Slot],
    fuzz: &mut CoreRng,
) -> Result<(), String> {
    for &slot in humans {
        if !authority.game().awaits_secret(slot) {
            continue;
        }
        if let Some(command) = G::arbitrary_secret_command(authority.game(), slot, fuzz) {
            let before = authority.seat_secret(slot);
            if authority.submit_secret(slot, &command).is_err()
                && authority.seat_secret(slot) != before
            {
                return Err(format!(
                    "rejected private command {command:?} changed the secret"
                ));
            }
        }
        if authority.game().awaits_secret(slot) {
            authority
                .submit_default_secret(slot)
                .map_err(|error| format!("the game's own private command was refused: {error}"))?;
        }
    }
    Ok(())
}

fn drive_turn<G: Conformance>(
    authority: &mut Authority<G>,
    humans: &[Slot],
    fuzz: &mut CoreRng,
) -> Result<(), String> {
    match authority.game().awaiting() {
        Awaiting::Turn(slot) if humans.contains(&slot) => {
            let mut accepted = false;
            for _ in 0..64 {
                if submit_arbitrary(authority, slot, fuzz)? {
                    accepted = true;
                    break;
                }
            }
            if !accepted {
                authority.drop_player(slot);
            }
        }
        Awaiting::Simultaneous => {
            for &slot in humans {
                submit_arbitrary(authority, slot, fuzz)?;
            }
            authority.run_bots();
        }
        _ => authority.run_bots(),
    }
    if !authority.has_pending_inputs()
        && let Awaiting::Turn(slot) = authority.game().awaiting()
    {
        authority.drop_player(slot);
    }
    Ok(())
}

/// Returns whether the command was accepted.
fn submit_arbitrary<G: Conformance>(
    authority: &mut Authority<G>,
    slot: Slot,
    fuzz: &mut CoreRng,
) -> Result<bool, String> {
    let command = G::arbitrary_command(authority.game(), slot, fuzz);
    let before = authority.game().state_hash();
    // Clients offer actions by asking first: the answer must be the one applying it gives.
    let checked = authority.check_command(slot, &command);
    let submitted = authority.submit(slot, command.clone());
    if checked != submitted {
        return Err(format!(
            "check_command said {checked:?} for {command:?}, applying it said {submitted:?}"
        ));
    }
    match submitted {
        Ok(()) => Ok(true),
        Err(_) if authority.game().state_hash() != before => {
            Err(format!("rejected command {command:?} changed the state"))
        }
        Err(_) => Ok(false),
    }
}
