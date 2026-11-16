use mg_core::{Awaiting, CoreRng, Game, Outcome, OutcomeKind, PlayerSpec, Slot, Status};

use super::support::{arrange, field_settings, humans, line, new_game, point, put_food};
use crate::{DeadSnakeBehavior, DeathReason, Direction, GameEnd, Snake, WallCollisionMode};

use Direction::{Down, Left, Right, Up};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;

fn game(players: &[PlayerSpec]) -> Snake {
    new_game(
        field_settings(
            20,
            20,
            WallCollisionMode::Death,
            DeadSnakeBehavior::Disappear,
        ),
        players,
    )
}

fn outcome(game: &Snake) -> Outcome {
    match game.status() {
        Status::Finished(outcome) => outcome,
        Status::InProgress => panic!("game still running"),
    }
}

/// Puts seat 0 one step from the top wall heading into it; other seats far away and safe.
fn doom_seat_zero(game: &mut Snake) {
    let mut layout = vec![(line(point(1, 0), Down, 3), Up)];
    for column in 1..game.seats.len() {
        let column = u16::try_from(column).unwrap();
        layout.push((line(point(column * 2 + 2, 10), Down, 3), Up));
    }
    arrange(game, &layout);
}

#[test]
fn solo_game_ends_with_defeat_when_the_snake_dies() {
    let mut game = game(&[HUMAN]);
    let head = game.seats[0].body[0];
    put_food(&mut game, point(head.x, head.y - 1));
    game.step();
    assert_eq!(game.status(), Status::InProgress);
    arrange(&mut game, &[(line(point(1, 0), Down, 3), Up)]);
    game.step();
    assert_eq!(
        outcome(&game),
        Outcome {
            kind: OutcomeKind::Defeat,
            winners: vec![],
            scores: vec![1],
        }
    );
    assert_eq!(
        game.end,
        Some(GameEnd {
            reason: DeathReason::WallCollision,
            winners: vec![],
        })
    );
    assert_eq!(game.awaiting(), Awaiting::Nobody);
}

#[test]
fn last_snake_standing_wins() {
    let mut game = game(&humans(2));
    doom_seat_zero(&mut game);
    game.step();
    assert_eq!(outcome(&game).kind, OutcomeKind::Winner);
    assert_eq!(outcome(&game).winners, [Slot::new(1)]);
}

#[test]
fn simultaneous_last_deaths_are_a_draw() {
    let mut game = game(&humans(2));
    arrange(
        &mut game,
        &[
            (line(point(4, 5), Left, 3), Right),
            (line(point(6, 5), Right, 3), Left),
        ],
    );
    game.step();
    assert_eq!(outcome(&game).kind, OutcomeKind::Draw);
    assert!(outcome(&game).winners.is_empty());
}

#[test]
fn surviving_bots_win_once_the_last_human_dies() {
    let mut game = game(&[HUMAN, BOT, BOT]);
    doom_seat_zero(&mut game);
    game.step();
    assert_eq!(
        outcome(&game),
        Outcome {
            kind: OutcomeKind::Winner,
            winners: vec![Slot::new(1), Slot::new(2)],
            scores: vec![0, 0, 0],
        }
    );
    assert_eq!(
        game.end,
        Some(GameEnd {
            reason: DeathReason::WallCollision,
            winners: vec![Slot::new(1), Slot::new(2)],
        })
    );
}

#[test]
fn a_sole_surviving_bot_wins_once_the_last_human_dies() {
    let mut game = game(&[HUMAN, BOT]);
    doom_seat_zero(&mut game);
    game.step();
    assert_eq!(outcome(&game).kind, OutcomeKind::Winner);
    assert_eq!(outcome(&game).winners, [Slot::new(1)]);
}

#[test]
fn humans_and_bots_dying_together_is_a_draw() {
    let mut game = game(&[HUMAN, BOT]);
    arrange(
        &mut game,
        &[
            (line(point(4, 5), Left, 3), Right),
            (line(point(6, 5), Right, 3), Left),
        ],
    );
    game.step();
    assert_eq!(outcome(&game).kind, OutcomeKind::Draw);
    assert!(outcome(&game).winners.is_empty());
}

#[test]
fn bots_only_tables_play_on_until_one_is_left() {
    let mut game = game(&[BOT, BOT, BOT]);
    doom_seat_zero(&mut game);
    game.step();
    assert_eq!(game.status(), Status::InProgress);
}

#[test]
fn dropping_a_player_kills_its_snake() {
    let mut game = game(&humans(2));
    game.drop_player(Slot::new(1));
    assert_eq!(game.seats[1].death, Some(DeathReason::PlayerDisconnected));
    assert_eq!(
        game.end,
        Some(GameEnd {
            reason: DeathReason::PlayerDisconnected,
            winners: vec![Slot::new(0)],
        })
    );
    let before = game.state_hash();
    game.drop_player(Slot::new(0));
    game.drop_player(Slot::new(9));
    assert_eq!(game.state_hash(), before);
}

#[test]
fn finished_games_are_frozen() {
    let mut game = game(&humans(2));
    game.drop_player(Slot::new(1));
    let before = game.state_hash();
    game.step();
    assert!(game.roll_world(&mut CoreRng::new(1)).is_empty());
    assert_eq!(game.state_hash(), before);
}
