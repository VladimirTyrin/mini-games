use mg_core::{Budget, CoreRng, Slot};

use crate::game::Snake;
use crate::io::Command;
use crate::model::{CellContent, Direction, SeatState};

/// The bot's snake when it should decide: game running, snake alive, nothing queued. Bots only
/// queue into an empty buffer, so their command is never `Busy` and always uses the current
/// direction as reference.
pub(crate) fn idle_seat(game: &Snake, me: Slot) -> Option<&SeatState> {
    game.seat(me)
        .filter(|seat| game.is_running() && seat.is_alive() && seat.queued_turns.is_empty())
}

/// Not leaving the field in `Death` mode and not entering a solid body. The own tail counts as
/// free (it always vacates: food never lies on a body, so the snake cannot be growing into it);
/// other tails do not, since their owner may grow. Head-on risks are ignored.
pub(crate) fn is_safe(game: &Snake, me: Slot, seat: &SeatState, direction: Direction) -> bool {
    let Some(target) = seat
        .head()
        .and_then(|head| game.field.neighbour(head, direction))
    else {
        return false;
    };
    match game.grid.get(target) {
        Some(CellContent::Empty | CellContent::Food) => true,
        Some(CellContent::Body(owner)) => owner == me && seat.tail() == Some(target),
        None => false,
    }
}

/// Non-reversing directions, the current one first (so ties keep going straight).
pub(crate) fn candidate_directions(seat: &SeatState) -> impl Iterator<Item = Direction> {
    let current = seat.direction;
    std::iter::once(current).chain(
        Direction::ALL
            .into_iter()
            .filter(move |&direction| direction != current && !direction.is_opposite(current)),
    )
}

/// `None` when the choice is to keep going straight.
pub(crate) fn turn_towards(seat: &SeatState, direction: Direction) -> Option<Command> {
    (direction != seat.direction).then_some(Command::Turn(direction))
}

/// Random behaviour: a uniformly random safe direction, else straight on.
pub(crate) fn random_safe_turn(
    game: &Snake,
    me: Slot,
    seat: &SeatState,
    rng: &mut CoreRng,
    budget: &mut Budget,
) -> Option<Command> {
    let safe: Vec<Direction> = candidate_directions(seat)
        .filter(|&direction| budget.spend(1) && is_safe(game, me, seat, direction))
        .collect();
    let choice = rng.pick(&safe).copied().unwrap_or(seat.direction);
    turn_towards(seat, choice)
}
