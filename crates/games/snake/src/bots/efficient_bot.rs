use mg_core::{Bot, Budget, CoreRng, Slot};

use crate::bots::{candidate_directions, idle_seat, is_safe, random_safe_turn, turn_towards};
use crate::game::Snake;
use crate::io::Command;
use crate::model::Point;

/// Greedy: one safe step that minimises the (wrap-aware) distance to the nearest food. Ties go
/// to the nearer food by `Point` order, then to going straight, then to `Direction::ALL` order.
/// Without food it behaves like the random bot (legacy drove straight into the wall).
#[derive(Debug)]
pub(crate) struct EfficientBot;

impl Bot<Snake> for EfficientBot {
    fn decide(
        &mut self,
        game: &Snake,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        let seat = idle_seat(game, me)?;
        let head = seat.head()?;
        let mut nearest: Option<(u32, Point)> = None;
        for &food in &game.food {
            if !budget.spend(1) {
                break;
            }
            let candidate = (game.field.distance(head, food), food);
            if nearest.is_none_or(|best| candidate < best) {
                nearest = Some(candidate);
            }
        }
        let Some((_, target)) = nearest else {
            return random_safe_turn(game, me, seat, rng, budget);
        };
        let mut best: Option<(u32, crate::model::Direction)> = None;
        for direction in candidate_directions(seat) {
            if !budget.spend(1) {
                break;
            }
            if !is_safe(game, me, seat, direction) {
                continue;
            }
            let Some(next) = game.field.neighbour(head, direction) else {
                continue;
            };
            let distance = game.field.distance(next, target);
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, direction));
            }
        }
        best.and_then(|(_, direction)| turn_towards(seat, direction))
    }
}
