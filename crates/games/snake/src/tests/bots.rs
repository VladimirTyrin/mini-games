use mg_core::{Budget, CoreRng, Game, HasBots, PlayerSpec, Slot};

use super::support::{arrange, field_settings, line, new_game, point, put_food};
use crate::{BotKind, Command, DeadSnakeBehavior, Direction, Snake, WallCollisionMode};

use Direction::{Down, Left, Right, Up};
use WallCollisionMode::{Death, WrapAround};

fn game(wall: WallCollisionMode, seats: usize) -> Snake {
    new_game(
        crate::Settings {
            max_food_count: 2,
            ..field_settings(20, 20, wall, DeadSnakeBehavior::Disappear)
        },
        &vec![PlayerSpec::BOT; seats],
    )
}

fn decide(kind: BotKind, game: &Snake, seed: u64) -> Option<Command> {
    let mut budget = Budget::new(Snake::default_bot_budget(kind));
    Snake::make_bot(kind).decide(game, Slot::new(0), &mut CoreRng::new(seed), &mut budget)
}

#[test]
fn efficient_steps_towards_the_nearest_food() {
    let mut game = game(Death, 1);
    put_food(&mut game, point(3, 10));
    put_food(&mut game, point(10, 0));
    assert_eq!(
        decide(BotKind::Efficient, &game, 1),
        Some(Command::Turn(Left))
    );
}

#[test]
fn efficient_keeps_going_when_food_is_ahead() {
    let mut game = game(Death, 1);
    put_food(&mut game, point(10, 3));
    assert_eq!(decide(BotKind::Efficient, &game, 1), None);
}

#[test]
fn efficient_measures_distance_around_wrapping_walls() {
    for (wall, expected) in [(WrapAround, Left), (Death, Right)] {
        let mut game = game(wall, 1);
        arrange(&mut game, &[(line(point(1, 10), Down, 3), Up)]);
        put_food(&mut game, point(18, 10));
        assert_eq!(
            decide(BotKind::Efficient, &game, 1),
            Some(Command::Turn(expected)),
            "{wall:?}"
        );
    }
}

/// Legacy Efficient kept its direction without food and drove into the wall.
#[test]
fn efficient_without_food_avoids_the_wall() {
    let mut game = game(Death, 1);
    arrange(&mut game, &[(line(point(5, 0), Down, 3), Up)]);
    for seed in 0..50 {
        let command = decide(BotKind::Efficient, &game, seed);
        assert!(
            matches!(command, Some(Command::Turn(Left | Right))),
            "seed {seed}: {command:?}"
        );
    }
}

#[test]
fn bots_wait_while_a_turn_is_queued() {
    let mut game = game(Death, 1);
    put_food(&mut game, point(3, 10));
    game.apply_command(Slot::new(0), &Command::Turn(Right))
        .unwrap();
    for kind in Snake::BOT_KINDS {
        assert_eq!(decide(*kind, &game, 1), None, "{kind:?}");
    }
}

#[test]
fn boxed_in_bots_keep_their_direction() {
    let mut game = game(Death, 2);
    let wall_around = vec![
        point(6, 5),
        point(6, 4),
        point(5, 4),
        point(4, 4),
        point(4, 5),
    ];
    arrange(
        &mut game,
        &[(line(point(5, 5), Down, 3), Up), (wall_around, Down)],
    );
    put_food(&mut game, point(15, 15));
    for kind in Snake::BOT_KINDS {
        assert_eq!(decide(*kind, &game, 1), None, "{kind:?}");
    }
}

#[test]
fn bot_commands_are_always_accepted() {
    let mut game = new_game(
        crate::Settings {
            max_food_count: 5,
            ..field_settings(16, 12, WrapAround, DeadSnakeBehavior::StayOnField)
        },
        &[PlayerSpec::BOT; 6],
    );
    let mut bots: Vec<_> = [BotKind::Efficient, BotKind::Random]
        .into_iter()
        .cycle()
        .take(6)
        .map(Snake::make_bot)
        .collect();
    let mut rng = CoreRng::new(77);
    for _ in 0..400 {
        if !game.is_running() {
            break;
        }
        for (slot, bot) in Slot::all(bots.len()).zip(bots.iter_mut()) {
            let mut budget = Budget::new(64);
            if let Some(command) = bot.decide(&game, slot, &mut rng, &mut budget) {
                game.apply_command(slot, &command).unwrap();
            }
        }
        for event in game.roll_world(&mut rng) {
            game.apply_world(&event).unwrap();
        }
        game.step();
    }
}
