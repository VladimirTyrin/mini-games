use mg_core::{CommandError, CoreRng, Game};

use super::support::{arrange, field_settings, humans, new_game, point, put_food};
use crate::{
    DeadSnakeBehavior, Direction, Point, RawSettings, Refusal, Snake, WallCollisionMode, WorldEvent,
};

fn game(max_food_count: u32, food_spawn_permille: u32) -> Snake {
    let settings = RawSettings {
        max_food_count,
        food_spawn_permille,
        ..field_settings(
            10,
            10,
            WallCollisionMode::WrapAround,
            DeadSnakeBehavior::Disappear,
        )
    };
    new_game(settings, &humans(1))
}

/// Row-by-row zigzag through the 10x10 field: a contiguous body of `length` cells.
fn zigzag(length: usize) -> Vec<Point> {
    (0..10u16)
        .flat_map(|row| {
            (0..10u16).map(move |step| {
                let column = if row % 2 == 0 { step } else { 9 - step };
                point(column, row)
            })
        })
        .take(length)
        .collect()
}

#[test]
fn at_most_one_food_per_tick_up_to_the_cap() {
    let mut game = game(50, 1000);
    let mut rng = CoreRng::new(5);
    for tick in 1..=60 {
        let events = game.roll_world(&mut rng);
        assert_eq!(events.len(), usize::from(tick <= 50), "tick {tick}");
        for event in &events {
            game.apply_world(event).unwrap();
        }
    }
    assert_eq!(game.food.len(), 50);
    let mut cells = game.food.clone();
    cells.sort();
    cells.dedup();
    assert_eq!(cells.len(), 50);
    assert!(
        game.seats[0]
            .body
            .iter()
            .all(|cell| !game.food.contains(cell))
    );
}

#[test]
fn spawn_chance_is_per_mille() {
    let game = game(1, 250);
    let mut rng = CoreRng::new(9);
    let spawned = (0..4000)
        .filter(|_| !game.roll_world(&mut rng).is_empty())
        .count();
    assert!((850..1150).contains(&spawned), "{spawned}");
}

#[test]
fn full_board_gets_no_food() {
    let mut game = game(1, 1000);
    arrange(&mut game, &[(zigzag(100), Direction::Left)]);
    let mut rng = CoreRng::new(3);
    for _ in 0..100 {
        assert!(game.roll_world(&mut rng).is_empty());
    }
}

#[test]
fn food_lands_uniformly_on_the_free_cells() {
    let mut game = game(1, 1000);
    arrange(&mut game, &[(zigzag(98), Direction::Left)]);
    let free = [point(1, 9), point(0, 9)];
    let mut rng = CoreRng::new(11);
    let mut hits = [0u32; 2];
    for _ in 0..2000 {
        let events = game.roll_world(&mut rng);
        let [WorldEvent::SpawnFood(cell)] = events.as_slice() else {
            panic!("expected one spawn, got {events:?}");
        };
        let slot = free.iter().position(|free_cell| free_cell == cell).unwrap();
        hits[slot] += 1;
    }
    assert!(
        hits.iter().all(|&count| (900..1100).contains(&count)),
        "{hits:?}"
    );
}

#[test]
fn invalid_spawns_are_rejected_without_side_effects() {
    let mut game = game(2, 1000);
    put_food(&mut game, point(0, 0));
    let body_cell = game.seats[0].body[1];
    let before = game.state_hash();
    for cell in [point(0, 0), body_cell, point(10, 3), point(3, 10)] {
        assert_eq!(
            game.apply_world(&WorldEvent::SpawnFood(cell)),
            Err(CommandError::Illegal(Refusal::FoodCellNotEmpty)),
            "{cell:?}"
        );
    }
    assert_eq!(game.state_hash(), before);
    put_food(&mut game, point(1, 0));
    assert_eq!(
        game.apply_world(&WorldEvent::SpawnFood(point(2, 0))),
        Err(CommandError::Illegal(Refusal::FoodCapReached))
    );
}
