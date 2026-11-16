use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{CoreRng, PlayerSpec, Slot};

use crate::{
    COLUMNS, Cell, CellIndex, CellPair, Command, Hint, HintMode, HintsLeft,
    LIMITED_HINTS_PER_REFILL, LIMITED_INITIAL_HINTS, NumbersMatch, Phase, RawSettings, Refills,
};

const HINT_MODES: [HintMode; 3] = [HintMode::Limited, HintMode::Unlimited, HintMode::Disabled];

impl Conformance for NumbersMatch {
    fn sample_setups() -> Vec<Setup<Self>> {
        HINT_MODES
            .iter()
            .flat_map(|&hint_mode| {
                (0..=Refills::MAX).map(move |refills| {
                    Setup::new(RawSettings { hint_mode, refills }, vec![PlayerSpec::HUMAN])
                })
            })
            .collect()
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        let too_many_refills = |refills| RawSettings {
            refills,
            ..RawSettings::default()
        };
        vec![
            Setup::new(too_many_refills(Refills::MAX + 1), vec![PlayerSpec::HUMAN]),
            Setup::new(too_many_refills(u32::MAX), vec![PlayerSpec::HUMAN]),
            Setup::new(RawSettings::default(), vec![]),
            Setup::new(RawSettings::default(), vec![PlayerSpec::HUMAN; 2]),
            Setup::new(RawSettings::default(), vec![PlayerSpec::BOT]),
        ]
    }

    /// Mostly the first legal pair (or a refill when stuck), sometimes noise. Unforced refills
    /// stay rare: each one can double the board, which makes long simulations crawl.
    fn arbitrary_command(game: &Self, _slot: Slot, rng: &mut CoreRng) -> Command {
        let roll = rng.below(100);
        if roll < 85 {
            return match game.find_pair() {
                Some(pair) if rng.chance(1, 2) => {
                    Command::RemovePair(CellPair::new(pair.second, pair.first))
                }
                Some(pair) => Command::RemovePair(pair),
                None => Command::Refill,
            };
        }
        if roll < 86 {
            return Command::Refill;
        }
        if roll < 92 {
            return Command::RequestHint;
        }
        let bound = u32::try_from(game.cells().len() + COLUMNS).unwrap_or(u32::MAX);
        Command::RemovePair(CellPair::new(
            CellIndex::new(rng.below(bound)),
            CellIndex::new(rng.below(bound)),
        ))
    }

    fn check_invariants(&self) -> Result<(), String> {
        let cells = self.cells();
        if !cells.len().is_multiple_of(COLUMNS) {
            return Err(format!("{} cells is not whole rows", cells.len()));
        }
        if self.phase() != Phase::Dealing
            && cells
                .chunks(COLUMNS)
                .any(|row| !row.iter().any(|cell| cell.is_active()))
        {
            return Err("a row without active cells survived".into());
        }
        let first_empty = cells
            .iter()
            .position(|cell| *cell == Cell::Empty)
            .unwrap_or(cells.len());
        if cells[first_empty..].iter().any(|cell| *cell != Cell::Empty) {
            return Err("empty cell before a value".into());
        }
        let stats = self.stats();
        if self.refills_left() + stats.refills_used != self.settings().refills.get() {
            return Err("refill counters out of sync".into());
        }
        let expected_hints = match self.settings().hint_mode {
            HintMode::Limited => HintsLeft::Count(
                LIMITED_INITIAL_HINTS + LIMITED_HINTS_PER_REFILL * stats.refills_used
                    - stats.hints_used,
            ),
            HintMode::Unlimited => HintsLeft::Unlimited,
            HintMode::Disabled => HintsLeft::Count(0),
        };
        if self.hints_left() != expected_hints {
            return Err(format!("hints left {:?}", self.hints_left()));
        }
        if let Some(Hint::Pair(pair)) = self.hint()
            && !self.can_remove_pair(pair)
        {
            return Err("hinted pair is not removable".into());
        }
        let active = cells.iter().filter(|cell| cell.is_active()).count();
        match self.phase() {
            Phase::Won if active != 0 => Err("won with active cells".into()),
            Phase::Playing if active == 0 => Err("playing on an empty board".into()),
            Phase::Playing if self.refills_left() == 0 && self.find_pair().is_none() => {
                Err("playing without moves".into())
            }
            _ => Ok(()),
        }
    }
}

#[test]
fn passes_conformance() {
    conformance::run_all::<NumbersMatch>(4);
}

#[test]
fn scripted_hash_is_pinned() {
    let setup = Setup::new(RawSettings::default(), vec![PlayerSpec::HUMAN]);
    assert_eq!(
        conformance::scripted_hash::<NumbersMatch>(&setup, 42),
        0x4C1B_E1E4_FB45_5D7E
    );
}
