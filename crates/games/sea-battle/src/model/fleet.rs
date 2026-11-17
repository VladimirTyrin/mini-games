use mg_core::{CoreRng, StateHasher};

use crate::model::{Cell, FleetError, Orientation, Ship};

/// The ship lengths of a fleet, longest first: the classic 1×4, 2×3, 3×2, 4×1.
pub const FLEET_LENGTHS: [u8; 10] = [4, 3, 3, 2, 2, 2, 1, 1, 1, 1];

/// Placement attempts for one ship before a random fleet starts over, and how often it may start
/// over. A classic fleet nearly always fits on the first try.
const RANDOM_TRIES_PER_SHIP: u32 = 200;
const RANDOM_RESTARTS: u32 = 50;

/// A complete, legal fleet: the classic ships, none touching another. Valid by construction
/// ([`Fleet::parse`]); the ships are kept sorted, so equal fleets compare equal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fleet {
    ships: Vec<Ship>,
}

impl Fleet {
    pub fn parse(ships: &[Ship]) -> Result<Self, FleetError> {
        let mut lengths: Vec<u8> = ships.iter().map(|ship| ship.length()).collect();
        lengths.sort_unstable_by(|a, b| b.cmp(a));
        if lengths != FLEET_LENGTHS {
            return Err(FleetError::WrongShips);
        }
        for (index, ship) in ships.iter().enumerate() {
            for other in ships.iter().skip(index + 1) {
                if other.cells().any(|cell| ship.touches(cell)) {
                    return Err(FleetError::ShipsTouch);
                }
            }
        }
        let mut ships = ships.to_vec();
        ships.sort_unstable();
        Ok(Self { ships })
    }

    /// A fleet placed at random, or `None` in the practically impossible case that every attempt
    /// boxed itself in.
    pub fn random(rng: &mut CoreRng) -> Option<Self> {
        (0..RANDOM_RESTARTS)
            .find_map(|_| Self::try_random(rng))
            .and_then(|ships| Self::parse(&ships).ok())
    }

    fn try_random(rng: &mut CoreRng) -> Option<Vec<Ship>> {
        let mut ships: Vec<Ship> = Vec::with_capacity(FLEET_LENGTHS.len());
        for length in FLEET_LENGTHS {
            let placed = (0..RANDOM_TRIES_PER_SHIP).find_map(|_| {
                let orientation = if rng.chance(1, 2) {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                };
                let x = u8::try_from(rng.below(u32::from(Cell::SIDE))).ok()?;
                let y = u8::try_from(rng.below(u32::from(Cell::SIDE))).ok()?;
                let ship = Ship::new(Cell::new(x, y)?, length, orientation)?;
                let fits = ships
                    .iter()
                    .all(|other| !other.cells().any(|cell| ship.touches(cell)));
                fits.then_some(ship)
            })?;
            ships.push(placed);
        }
        Some(ships)
    }

    pub fn ships(&self) -> &[Ship] {
        &self.ships
    }

    pub fn ship_at(&self, cell: Cell) -> Option<Ship> {
        self.ships.iter().copied().find(|ship| ship.covers(cell))
    }

    pub(crate) fn hash(&self, hasher: &mut StateHasher) {
        hasher.length(self.ships.len());
        for ship in &self.ships {
            ship.hash(hasher);
        }
    }
}
