use std::collections::BTreeMap;

use mg_core::Slot;

use crate::model::{Direction, Field, SeatState};
use crate::rules::DeathReason;
use crate::settings::WallCollisionMode;
use crate::view::{Crash, CrashPoint};

/// One step of `direction` in half cells.
fn half_step(direction: Direction) -> (i64, i64) {
    match direction {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    }
}

/// A dead snake keeps the body it had before the fatal move and that move's direction, so it hit
/// the edge of the cell in front of its head. Heads aiming at one cell met in that cell; heads
/// swapping cells meet on the edge between them, which both reach from their own side (across a
/// wrap-around border too, once wrapped).
pub(crate) fn crashes(field: Field, seats: &[SeatState]) -> Vec<Crash> {
    let crashed: Vec<(Slot, &SeatState)> = Slot::all(seats.len())
        .zip(seats)
        .filter(|(_, seat)| {
            matches!(
                seat.death,
                Some(
                    DeathReason::WallCollision
                        | DeathReason::SelfCollision
                        | DeathReason::SnakeCollision
                )
            )
        })
        .collect();
    let mut aimed_at: BTreeMap<(u16, u16), usize> = BTreeMap::new();
    for (_, seat) in &crashed {
        if seat.death != Some(DeathReason::SnakeCollision) {
            continue;
        }
        if let Some(target) = seat
            .head()
            .and_then(|head| field.neighbour(head, seat.direction))
        {
            *aimed_at.entry((target.x, target.y)).or_insert(0) += 1;
        }
    }
    let wraps = field.wall_mode() == WallCollisionMode::WrapAround;
    let width2 = 2 * i64::from(field.width());
    let height2 = 2 * i64::from(field.height());
    let mut by_point: BTreeMap<CrashPoint, Vec<Slot>> = BTreeMap::new();
    for (slot, seat) in crashed {
        let Some(head) = seat.head() else { continue };
        let contested = (seat.death == Some(DeathReason::SnakeCollision))
            .then(|| field.neighbour(head, seat.direction))
            .flatten()
            .filter(|target| aimed_at.get(&(target.x, target.y)).copied().unwrap_or(0) > 1);
        let (x2, y2) = match contested {
            Some(target) => (2 * i64::from(target.x) + 1, 2 * i64::from(target.y) + 1),
            None => {
                let (dx, dy) = half_step(seat.direction);
                let (x2, y2) = (
                    2 * i64::from(head.x) + 1 + dx,
                    2 * i64::from(head.y) + 1 + dy,
                );
                if wraps {
                    (x2.rem_euclid(width2), y2.rem_euclid(height2))
                } else {
                    (x2, y2)
                }
            }
        };
        let (Ok(x2), Ok(y2)) = (u32::try_from(x2), u32::try_from(y2)) else {
            continue;
        };
        by_point
            .entry(CrashPoint { x2, y2 })
            .or_default()
            .push(slot);
    }
    by_point
        .into_iter()
        .map(|(at, slots)| Crash {
            at,
            head_on: slots.len() > 1,
            slots,
        })
        .collect()
}
