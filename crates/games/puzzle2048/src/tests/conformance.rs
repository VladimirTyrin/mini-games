use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{CoreRng, Game, PlayerSpec, Slot, Viewer};

use super::support::settings;
use crate::{Command, Direction, Phase, Puzzle2048, Settings, TileId, TileOrigin, TileValue};

fn human(settings: Settings) -> Setup<Puzzle2048> {
    Setup::new(settings, vec![PlayerSpec::HUMAN])
}

impl Conformance for Puzzle2048 {
    fn sample_setups() -> Vec<Setup<Self>> {
        [
            settings(2, 2, 8),
            settings(4, 4, 2048),
            settings(10, 10, 65_536),
            settings(3, 5, 16),
            settings(5, 3, 32),
            settings(2, 10, 64),
            settings(3, 3, 128),
        ]
        .into_iter()
        .map(human)
        .collect()
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        let mut setups = Vec::new();
        for size in [0, 1, 11, u32::MAX] {
            setups.push(human(settings(size, 4, 2048)));
            setups.push(human(settings(4, size, 2048)));
        }
        for target in [0, 2, 4, 6, 12, 2047, 131_072, 1 << 31, u32::MAX] {
            setups.push(human(settings(4, 4, target)));
        }
        setups.push(Setup::new(Settings::default(), Vec::new()));
        setups.push(Setup::new(
            Settings::default(),
            vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
        ));
        setups.push(Setup::new(Settings::default(), vec![PlayerSpec::BOT]));
        setups
    }

    fn arbitrary_command(_game: &Self, _slot: Slot, rng: &mut CoreRng) -> Command {
        Command::Move(*rng.pick(&Direction::ALL).unwrap())
    }

    fn check_invariants(&self) -> Result<(), String> {
        let view = self.view(Viewer::Spectator);
        let cells: Vec<_> = view.tiles.iter().map(|tile| tile.cell).collect();
        if cells.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(format!("tiles not in strict row-major order: {cells:?}"));
        }
        if let Some(tile) = view
            .tiles
            .iter()
            .find(|tile| tile.cell.row >= view.height || tile.cell.col >= view.width)
        {
            return Err(format!("tile outside the board: {tile:?}"));
        }
        if let Some(tile) = view.tiles.iter().find(|tile| {
            TileValue::new(tile.value.get()).is_none() || tile.value.get() > view.target.get()
        }) {
            return Err(format!("bad tile value: {tile:?}"));
        }
        let mut ids: Vec<TileId> = view.tiles.iter().map(|tile| tile.id).collect();
        ids.sort();
        ids.dedup();
        if ids.len() != view.tiles.len() {
            return Err("duplicate tile ids".into());
        }
        let spawned = view
            .tiles
            .iter()
            .filter(|tile| tile.origin == TileOrigin::Spawned)
            .count();
        let expected_spawned = if view.moves == 0 { 2 } else { 1 };
        if spawned != expected_spawned {
            return Err(format!(
                "{spawned} spawned tiles after {} moves",
                view.moves
            ));
        }
        let reached = view
            .highest_tile
            .is_some_and(|tile| view.target.is_reached_by(tile));
        let can_move = |game: &Puzzle2048| Direction::ALL.iter().any(|&d| game.can_move(d));
        match view.phase {
            Phase::Won if !reached => Err("won without reaching the target".into()),
            Phase::Playing if reached => Err("target reached but still playing".into()),
            Phase::Playing if !can_move(self) => Err("playing without a legal move".into()),
            Phase::Lost => {
                let replica = Puzzle2048::with_values(
                    &settings(
                        u32::from(view.width),
                        u32::from(view.height),
                        view.target.get(),
                    ),
                    &view.cell_values(),
                );
                if can_move(&replica) {
                    Err("lost although a move is possible".into())
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

#[test]
fn passes_conformance() {
    conformance::run_all::<Puzzle2048>(12);
}

#[test]
fn scripted_hash_is_pinned() {
    let hash = conformance::scripted_hash(&human(Settings::default()), 2048);
    assert_eq!(hash, GOLDEN_HASH, "got {hash:#018x}");
}

/// Changes only with a deliberate rule or hashing change; also compared on wasm32.
const GOLDEN_HASH: u64 = 0x7d2f_ed5c_2bae_d633;
