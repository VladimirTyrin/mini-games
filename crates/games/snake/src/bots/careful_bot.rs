use mg_core::{Bot, Budget, CoreRng, Slot};

use crate::bots::{candidate_directions, idle_seat, is_safe, turn_towards};
use crate::game::Snake;
use crate::io::Command;
use crate::model::{CellContent, Direction, Point, SeatState};

/// Efficient, minding the other snakes: of the safe steps it keeps off cells another head may
/// enter next tick (a head-on crash kills both) and out of pockets too small for its own length,
/// and among what is left heads for the nearest food. When every step is risky it takes the
/// least bad one. Without food it keeps to the safest, roomiest way, straight on when tied.
#[derive(Debug)]
pub(crate) struct CarefulBot;

/// How a step looks, worst first in each field: compared as a tuple, the largest wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StepRating {
    clear_of_heads: bool,
    room_enough: bool,
    room: usize,
    /// Negated distance to the nearest food: nearer is larger.
    food_closeness: i64,
}

impl Bot<Snake> for CarefulBot {
    fn decide(
        &mut self,
        game: &Snake,
        me: Slot,
        _rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        let seat = idle_seat(game, me)?;
        let head = seat.head()?;
        let contested = cells_other_heads_may_enter(game, me);
        let length = seat.body.len();
        let mut best: Option<(StepRating, Direction)> = None;
        for direction in candidate_directions(seat) {
            if !budget.spend(1) || !is_safe(game, me, seat, direction) {
                continue;
            }
            let Some(next) = game.field.neighbour(head, direction) else {
                continue;
            };
            let room = room_from(game, me, seat, next, length, budget);
            let nearest_food = game
                .food
                .iter()
                .map(|&food| game.field.distance(next, food))
                .min()
                .unwrap_or(0);
            let rating = StepRating {
                clear_of_heads: !contested.contains(&next),
                room_enough: room >= length,
                room,
                food_closeness: -i64::from(nearest_food),
            };
            // Ties keep the earlier candidate: straight on first.
            if best.is_none_or(|(best_rating, _)| rating > best_rating) {
                best = Some((rating, direction));
            }
        }
        best.and_then(|(_, direction)| turn_towards(seat, direction))
    }
}

/// Where the other living heads may be next tick: their queued turn if they have one, otherwise
/// any way but back (they may still turn before it).
fn cells_other_heads_may_enter(game: &Snake, me: Slot) -> Vec<Point> {
    let mut cells = Vec::new();
    for (slot, seat) in Slot::all(game.seats.len()).zip(&game.seats) {
        if slot == me || !seat.is_alive() {
            continue;
        }
        let Some(head) = seat.head() else { continue };
        let directions: Vec<Direction> = match seat.queued_turns.first() {
            Some(&queued) => vec![queued],
            None => candidate_directions(seat).collect(),
        };
        cells.extend(
            directions
                .into_iter()
                .filter_map(|direction| game.field.neighbour(head, direction)),
        );
    }
    cells
}

/// Free cells reachable from `start` (counted up to `enough`, more is not needed): the pocket the
/// snake would be heading into. Its own tail counts as free, as it moves away.
fn room_from(
    game: &Snake,
    me: Slot,
    seat: &SeatState,
    start: Point,
    enough: usize,
    budget: &mut Budget,
) -> usize {
    let free = |point: Point| match game.grid.get(point) {
        Some(CellContent::Empty | CellContent::Food) => true,
        Some(CellContent::Body(owner)) => owner == me && seat.tail() == Some(point),
        None => false,
    };
    let mut seen = vec![false; game.field.cell_count()];
    let mut pending = vec![start];
    if let Some(cell) = game
        .field
        .index_of(start)
        .and_then(|index| seen.get_mut(index))
    {
        *cell = true;
    }
    let mut room = 0;
    while let Some(point) = pending.pop() {
        if room >= enough || !budget.spend(1) {
            break;
        }
        room += 1;
        for direction in Direction::ALL {
            let Some(next) = game.field.neighbour(point, direction) else {
                continue;
            };
            let Some(index) = game.field.index_of(next) else {
                continue;
            };
            let Some(visited) = seen.get_mut(index) else {
                continue;
            };
            if *visited || !free(next) {
                continue;
            }
            *visited = true;
            pending.push(next);
        }
    }
    room
}
