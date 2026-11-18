use mg_core::{Budget, CoreRng, Game, HasBots, PlayerSpec, Slot};

use super::support::{arrange, field_settings, line, new_game, point, put_food};
use crate::{BotKind, Command, DeadSnakeBehavior, Direction, Snake, WallCollisionMode};

use Direction::{Down, Left, Right, Up};
use WallCollisionMode::{Death, WrapAround};

fn game(wall: WallCollisionMode, seats: usize) -> Snake {
    new_game(
        crate::RawSettings {
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
        crate::RawSettings {
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

#[test]
fn careful_keeps_off_a_cell_another_head_may_enter() {
    let mut game = game(Death, 2);
    arrange(
        &mut game,
        &[
            (line(point(5, 10), Left, 3), Right),
            // Its head right below the cell ahead of ours: it may go up into it.
            (line(point(6, 11), Down, 3), Up),
        ],
    );
    put_food(&mut game, point(7, 10));
    assert_eq!(
        decide(BotKind::Efficient, &game, 1),
        None,
        "efficient goes for the food"
    );
    // Down is where the other head may turn too; up is clear.
    assert_eq!(decide(BotKind::Careful, &game, 1), Some(Command::Turn(Up)));
}

#[test]
fn careful_does_not_go_into_a_pocket_shorter_than_itself() {
    let mut game = game(Death, 2);
    arrange(
        &mut game,
        &[
            (line(point(5, 5), Left, 6), Right),
            // A U of the other body around (6, 5) and (7, 5), its head at the top going up.
            (
                vec![
                    point(6, 4),
                    point(7, 4),
                    point(8, 4),
                    point(8, 5),
                    point(8, 6),
                    point(7, 6),
                    point(6, 6),
                ],
                Up,
            ),
        ],
    );
    put_food(&mut game, point(7, 5));
    assert_eq!(
        decide(BotKind::Efficient, &game, 1),
        None,
        "efficient goes for the food"
    );
    // Up is next to the other head; down leads into open field.
    assert_eq!(
        decide(BotKind::Careful, &game, 1),
        Some(Command::Turn(Down))
    );
}

/// Two bots on a small field until one is left (or 2000 steps): the surviving slot, if any.
fn duel(first: BotKind, second: BotKind, seed: u64) -> Option<Slot> {
    let settings = <crate::Settings as mg_core::GameSettings>::parse(crate::RawSettings {
        max_food_count: 3,
        ..field_settings(12, 12, Death, DeadSnakeBehavior::StayOnField)
    })
    .unwrap();
    let bots = [(Slot::new(0), first), (Slot::new(1), second)]
        .into_iter()
        .map(|(slot, kind)| mg_core::BotSeat {
            slot,
            bot: Snake::make_bot(kind),
            budget_nodes: Snake::default_bot_budget(kind),
        })
        .collect();
    let mut authority =
        mg_core::Authority::<Snake>::new(&settings, &[PlayerSpec::BOT; 2], seed, bots).unwrap();
    for _ in 0..2000 {
        authority.run_bots();
        authority.seal_frame();
        if authority.game().status().is_finished() {
            break;
        }
    }
    assert_eq!(authority.rejected_bot_commands(), 0);
    authority
        .game()
        .end
        .as_ref()
        .and_then(|end| end.winners.first().copied())
}

#[test]
fn careful_outlives_efficient_most_of_the_time() {
    let mut careful = 0;
    let mut efficient = 0;
    for seed in 0..40 {
        // Seats alternate, so where a snake starts does not decide it.
        let (careful_slot, winner) = if seed % 2 == 0 {
            (
                Slot::new(0),
                duel(BotKind::Careful, BotKind::Efficient, seed),
            )
        } else {
            (
                Slot::new(1),
                duel(BotKind::Efficient, BotKind::Careful, seed),
            )
        };
        match winner {
            Some(slot) if slot == careful_slot => careful += 1,
            Some(_) => efficient += 1,
            None => {}
        }
    }
    assert!(
        careful > 2 * efficient,
        "careful {careful}, efficient {efficient}"
    );
}
