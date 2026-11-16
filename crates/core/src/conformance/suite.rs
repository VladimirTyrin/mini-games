use super::contract::Conformance;
use super::setup::Setup;
use crate::authority::Authority;
use crate::awaiting::Awaiting;
use crate::compact_log::expand_compact_log;
use crate::matcher::Match;
use crate::player::PlayerSpec;
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
        if G::new(&setup.settings, &setup.players).is_ok() {
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
    let mut authority = Authority::<G>::new(&setup.settings, players, seed, G::bots_for(players))
        .map_err(|e| format!("sample setup rejected: {e}"))?;
    let mut mirror =
        Match::<G>::new(&setup.settings, players).map_err(|e| format!("mirror: {e}"))?;
    let mut fuzz = CoreRng::derive(seed, 0xF022);
    let humans: Vec<Slot> = Slot::all(players.len())
        .filter(|s| players[s.index()].is_human())
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

    let mut replay =
        Match::<G>::new(&setup.settings, players).map_err(|e| format!("replay: {e}"))?;
    let frames = expand_compact_log(authority.compact_log(), authority.next_tick());
    replay
        .apply_frames(&frames)
        .map_err(|e| format!("replay: {e}"))?;
    let final_hash = authority.game().state_hash();
    if replay.game().state_hash() != final_hash {
        return Err("replayed log diverged from the authority".into());
    }
    Ok(final_hash)
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
    match authority.submit(slot, command.clone()) {
        Ok(()) => Ok(true),
        Err(_) if authority.game().state_hash() != before => {
            Err(format!("rejected command {command:?} changed the state"))
        }
        Err(_) => Ok(false),
    }
}
