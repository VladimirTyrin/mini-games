use crate::model::{Cell, CellMark, Fleet, ShotOutcome, Waters};

/// What a shot at `cell` finds in `fleet`, given what `waters` already show: a ship is sunk when
/// every other cell of it was hit before.
pub(crate) fn resolve_shot(fleet: &Fleet, waters: &Waters, cell: Cell) -> ShotOutcome {
    let Some(ship) = fleet.ship_at(cell) else {
        return ShotOutcome::Miss;
    };
    let rest_hit = ship
        .cells()
        .filter(|own| *own != cell)
        .all(|own| waters.mark(own) == CellMark::Hit);
    if rest_hit {
        ShotOutcome::Sunk(ship)
    } else {
        ShotOutcome::Hit
    }
}
