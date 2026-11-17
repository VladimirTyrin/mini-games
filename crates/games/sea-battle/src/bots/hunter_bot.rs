use mg_core::{Bot, Budget, CoreRng, Slot};

use crate::bots::{BotKind, ship_density};
use crate::game::SeaBattle;
use crate::io::{Command, SecretCommand};
use crate::model::{Cell, CellMark, Fleet, Waters};

/// Random fleets the hard bot compares before keeping the one hardest to find.
const HARD_PLACEMENT_CANDIDATES: u32 = 24;

/// Fires at the opponent's sea from what everyone sees there; it never learns the fleet before
/// the end. Its level decides how well it hunts (see [`BotKind`]).
#[derive(Debug)]
pub struct HunterBot {
    kind: BotKind,
}

impl HunterBot {
    pub fn new(kind: BotKind) -> Self {
        Self { kind }
    }

    fn target(&self, waters: &Waters, rng: &mut CoreRng) -> Option<Cell> {
        let open: Vec<Cell> = Cell::all()
            .filter(|cell| waters.mark(*cell).is_open())
            .collect();
        let wounded: Vec<Cell> = Cell::all()
            .filter(|cell| waters.mark(*cell) == CellMark::Hit)
            .collect();
        match self.kind {
            BotKind::Easy => {
                // Half the time it forgets the wounded ship and fires anywhere.
                if !wounded.is_empty() && rng.chance(1, 2) {
                    let near = sides_of(&wounded, waters);
                    if let Some(cell) = rng.pick(&near) {
                        return Some(*cell);
                    }
                }
                rng.pick(&open).copied()
            }
            BotKind::Medium => {
                if !wounded.is_empty() {
                    let line = line_ends(&wounded, waters);
                    let near = if line.is_empty() {
                        sides_of(&wounded, waters)
                    } else {
                        line
                    };
                    if let Some(cell) = rng.pick(&near) {
                        return Some(*cell);
                    }
                }
                // Every ship of two or more covers a cell of one colour of the checkerboard.
                let diagonal: Vec<Cell> = open
                    .iter()
                    .copied()
                    .filter(|cell| (cell.x() + cell.y()) % 2 == 0)
                    .collect();
                rng.pick(if diagonal.is_empty() {
                    &open
                } else {
                    &diagonal
                })
                .copied()
            }
            BotKind::Hard => {
                let density = ship_density(waters);
                let best = open
                    .iter()
                    .filter_map(|cell| density.get(cell.index()).copied())
                    .max()
                    .unwrap_or(0);
                let best_cells: Vec<Cell> = open
                    .iter()
                    .copied()
                    .filter(|cell| density.get(cell.index()).copied() == Some(best))
                    .collect();
                rng.pick(&best_cells).or_else(|| rng.pick(&open)).copied()
            }
        }
    }

    /// The hard bot keeps, of a few random fleets, the one standing least where a hunter looks
    /// first on an empty sea (the middle).
    fn hard_fleet(rng: &mut CoreRng) -> Option<Fleet> {
        let empty_density = ship_density(&Waters::default());
        let exposure = |fleet: &Fleet| -> u32 {
            fleet
                .ships()
                .iter()
                .flat_map(|ship| ship.cells())
                .filter_map(|cell| empty_density.get(cell.index()).copied())
                .sum()
        };
        (0..HARD_PLACEMENT_CANDIDATES)
            .filter_map(|_| Fleet::random(rng))
            .min_by_key(exposure)
    }
}

/// Open cells beside the wounded ones.
fn sides_of(wounded: &[Cell], waters: &Waters) -> Vec<Cell> {
    let mut near: Vec<Cell> = wounded
        .iter()
        .flat_map(|cell| cell.sides())
        .filter(|cell| waters.mark(*cell).is_open())
        .collect();
    near.sort_unstable();
    near.dedup();
    near
}

/// With two or more hits in a row, the open cells just past either end of the row.
fn line_ends(wounded: &[Cell], waters: &Waters) -> Vec<Cell> {
    let (Some(first), Some(last)) = (wounded.iter().min(), wounded.iter().max()) else {
        return Vec::new();
    };
    if wounded.len() < 2 {
        return Vec::new();
    }
    let ends = if wounded.iter().all(|cell| cell.y() == first.y()) {
        [first.offset(-1, 0), last.offset(1, 0)]
    } else if wounded.iter().all(|cell| cell.x() == first.x()) {
        [first.offset(0, -1), last.offset(0, 1)]
    } else {
        return Vec::new();
    };
    ends.into_iter()
        .flatten()
        .filter(|cell| waters.mark(*cell).is_open())
        .collect()
}

impl Bot<SeaBattle> for HunterBot {
    fn decide(
        &mut self,
        game: &SeaBattle,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        budget.spend(1);
        let opponent = if me == Slot::new(0) {
            Slot::new(1)
        } else {
            Slot::new(0)
        };
        let waters = game.waters(opponent)?;
        self.target(waters, rng).map(Command::Fire)
    }

    fn decide_secret(
        &mut self,
        _game: &SeaBattle,
        _me: Slot,
        rng: &mut CoreRng,
    ) -> Option<SecretCommand> {
        let fleet = match self.kind {
            BotKind::Easy | BotKind::Medium => Fleet::random(rng),
            BotKind::Hard => Self::hard_fleet(rng),
        }?;
        Some(SecretCommand::PlaceFleet(fleet.ships().to_vec()))
    }
}
