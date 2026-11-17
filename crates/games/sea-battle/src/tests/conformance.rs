use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{BotSeat, CoreRng, HasBots, PlayerSpec, Slot};

use crate::{BotKind, Cell, Command, Orientation, RawSettings, SeaBattle, SecretCommand, Ship};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;

fn random_cell(rng: &mut CoreRng) -> Cell {
    let x = u8::try_from(rng.below(10)).unwrap();
    let y = u8::try_from(rng.below(10)).unwrap();
    Cell::new(x, y).unwrap()
}

impl Conformance for SeaBattle {
    fn sample_setups() -> Vec<Setup<Self>> {
        vec![
            Setup::new(RawSettings, vec![HUMAN, HUMAN]),
            Setup::new(RawSettings, vec![HUMAN, BOT]),
            Setup::new(RawSettings, vec![BOT, HUMAN]),
            // The bot arena: bots alone.
            Setup::new(RawSettings, vec![BOT, BOT]),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        vec![
            Setup::new(RawSettings, vec![]),
            Setup::new(RawSettings, vec![HUMAN]),
            Setup::new(RawSettings, vec![HUMAN; 3]),
        ]
    }

    /// Bot levels by seat, so the levels meet each other.
    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Self>> {
        const LEVELS: [BotKind; 2] = [BotKind::Hard, BotKind::Easy];
        Slot::all(players.len())
            .zip(LEVELS)
            .filter(|(slot, _)| {
                players
                    .get(slot.index())
                    .is_some_and(|spec| !spec.is_human())
            })
            .map(|(slot, kind)| BotSeat {
                slot,
                bot: SeaBattle::make_bot(kind),
                budget_nodes: SeaBattle::default_bot_budget(kind),
            })
            .collect()
    }

    /// Mostly shots at cells not yet known, sometimes at any cell, rarely giving up.
    fn arbitrary_command(game: &Self, slot: Slot, rng: &mut CoreRng) -> Command {
        if rng.below(150) == 0 {
            return Command::GiveUp;
        }
        let opponent = Slot::new(1 - slot.get().min(1));
        let open: Vec<Cell> = game
            .waters(opponent)
            .map(|waters| {
                Cell::all()
                    .filter(|cell| waters.mark(*cell).is_open())
                    .collect()
            })
            .unwrap_or_default();
        match rng.pick(&open) {
            Some(cell) if rng.below(8) != 0 => Command::Fire(*cell),
            _ => Command::Fire(random_cell(rng)),
        }
    }

    /// Now and then a heap of ships that is no fleet; otherwise nothing, and the game places a
    /// random fleet.
    fn arbitrary_secret_command(
        _game: &Self,
        _slot: Slot,
        rng: &mut CoreRng,
    ) -> Option<SecretCommand> {
        if rng.chance(1, 2) {
            return None;
        }
        let count = rng.below(12);
        let ships = (0..count)
            .filter_map(|_| {
                let length = u8::try_from(rng.range_inclusive(1, 4)).unwrap();
                Ship::new(random_cell(rng), length, Orientation::Vertical)
            })
            .collect();
        Some(SecretCommand::PlaceFleet(ships))
    }
}

#[test]
fn passes_conformance() {
    conformance::run_all::<SeaBattle>(20);
}

/// Pins the rules: if this value changes, a rule changed — update it deliberately.
const GOLDEN_HASH: u64 = 0x5A7D_814E_4EB2_D83E;

#[test]
fn scripted_hash_is_pinned() {
    let setup = Setup::<SeaBattle>::new(RawSettings, vec![HUMAN, HUMAN]);
    let hash = conformance::scripted_hash(&setup, 42);
    assert_eq!(hash, GOLDEN_HASH, "pin GOLDEN_HASH = {hash:#018x}");
}
