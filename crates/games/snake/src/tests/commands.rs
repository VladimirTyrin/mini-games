use mg_core::{CommandError, Game, Slot};

use super::support::{field_settings, humans, new_game, point, snake};
use crate::{Command, DeadSnakeBehavior, Direction, Snake, WallCollisionMode};

fn game() -> Snake {
    new_game(
        field_settings(
            20,
            20,
            WallCollisionMode::Death,
            DeadSnakeBehavior::Disappear,
        ),
        &humans(2),
    )
}

fn turn(game: &mut Snake, direction: Direction) -> Result<(), CommandError> {
    game.apply_command(Slot::new(0), &Command::Turn(direction))
}

#[test]
fn reversal_and_repeat_are_checked_against_the_last_queued_turn() {
    let mut game = game();
    assert_eq!(
        turn(&mut game, Direction::Down),
        Err(CommandError::Illegal("cannot reverse"))
    );
    assert_eq!(
        turn(&mut game, Direction::Up),
        Err(CommandError::Illegal("already heading that way"))
    );
    assert_eq!(turn(&mut game, Direction::Left), Ok(()));
    assert_eq!(
        turn(&mut game, Direction::Left),
        Err(CommandError::Illegal("already heading that way"))
    );
    assert_eq!(
        turn(&mut game, Direction::Right),
        Err(CommandError::Illegal("cannot reverse"))
    );
    assert_eq!(turn(&mut game, Direction::Down), Ok(()));
    assert_eq!(
        snake(&game, 0).queued_turns,
        [Direction::Left, Direction::Down]
    );
}

#[test]
fn full_buffer_is_busy_and_each_tick_consumes_one_turn() {
    let mut game = game();
    let start = snake(&game, 0).body[0];
    turn(&mut game, Direction::Left).unwrap();
    turn(&mut game, Direction::Down).unwrap();
    assert_eq!(turn(&mut game, Direction::Right), Err(CommandError::Busy));

    game.step();
    let after_one = snake(&game, 0);
    assert_eq!(after_one.direction, Direction::Left);
    assert_eq!(after_one.body[0], point(start.x - 1, start.y));
    assert_eq!(after_one.queued_turns, [Direction::Down]);
    assert_eq!(turn(&mut game, Direction::Right), Ok(()));

    game.step();
    let after_two = snake(&game, 0);
    assert_eq!(after_two.direction, Direction::Down);
    assert_eq!(after_two.body[0], point(start.x - 1, start.y + 1));
}

#[test]
fn rejected_commands_leave_the_state_untouched() {
    let mut game = game();
    turn(&mut game, Direction::Left).unwrap();
    turn(&mut game, Direction::Up).unwrap();
    let before = game.state_hash();
    for direction in Direction::ALL {
        assert!(turn(&mut game, direction).is_err());
    }
    assert_eq!(
        game.apply_command(Slot::new(7), &Command::Turn(Direction::Left)),
        Err(CommandError::NotPlaying)
    );
    assert_eq!(game.state_hash(), before);
}

#[test]
fn dead_snakes_and_finished_games_refuse_turns() {
    let mut game = new_game(
        field_settings(
            20,
            20,
            WallCollisionMode::Death,
            DeadSnakeBehavior::Disappear,
        ),
        &humans(3),
    );
    game.drop_player(Slot::new(0));
    assert_eq!(
        turn(&mut game, Direction::Left),
        Err(CommandError::NotPlaying)
    );
    game.drop_player(Slot::new(1));
    assert_eq!(
        game.apply_command(Slot::new(2), &Command::Turn(Direction::Left)),
        Err(CommandError::GameOver)
    );
}
