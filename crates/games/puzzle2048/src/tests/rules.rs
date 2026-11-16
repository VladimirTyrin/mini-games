use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, OutcomeKind, PlayerSpec, SetupError, Slot, Status,
};

use super::support::{
    PLAYER, board, deliver_spawns, play, settings, spawn, started, tile_count, try_move, values,
    view,
};
use crate::{Command, Direction, Phase, Puzzle2048, Settings, SpawnValue, WorldEvent};

fn create(settings: &Settings) -> Result<Puzzle2048, SetupError> {
    Puzzle2048::new(settings, &[PlayerSpec::HUMAN])
}

fn rejected_field(settings: &Settings) -> &'static str {
    match create(settings) {
        Err(SetupError::InvalidSetting { field, .. }) => field,
        other => panic!("{settings:?} must be rejected, got {other:?}"),
    }
}

#[test]
fn default_settings_match_the_web_defaults() {
    assert_eq!(Settings::default(), settings(4, 4, 2048));
    assert!(create(&Settings::default()).is_ok());
}

#[test]
fn board_size_limits() {
    for size in [Settings::MIN_SIZE, Settings::MAX_SIZE] {
        assert!(create(&settings(size, size, 2048)).is_ok());
    }
    for size in [0, 1, 11, u32::MAX] {
        assert_eq!(rejected_field(&settings(size, 4, 2048)), "width");
        assert_eq!(rejected_field(&settings(4, size, 2048)), "height");
    }
}

#[test]
fn target_is_a_bounded_power_of_two() {
    for target in [8, 16, 2048, 65_536] {
        assert!(create(&settings(4, 4, target)).is_ok(), "{target}");
    }
    for target in [0, 2, 4, 12, 2047, 131_072, 1 << 31, u32::MAX] {
        assert_eq!(
            rejected_field(&settings(4, 4, target)),
            "target",
            "{target}"
        );
    }
}

#[test]
fn exactly_one_human_seat() {
    let defaults = Settings::default();
    assert!(matches!(
        Puzzle2048::new(&defaults, &[]),
        Err(SetupError::PlayerCount { got: 0, .. })
    ));
    assert!(matches!(
        Puzzle2048::new(&defaults, &[PlayerSpec::HUMAN, PlayerSpec::HUMAN]),
        Err(SetupError::PlayerCount { got: 2, .. })
    ));
    assert!(matches!(
        Puzzle2048::new(&defaults, &[PlayerSpec::BOT]),
        Err(SetupError::InvalidPlayers(_))
    ));
}

#[test]
fn turn_based_and_waits_for_the_player() {
    let game = started(&Settings::default(), &mut CoreRng::new(1));
    assert_eq!(game.clock(), Clock::TurnBased);
    assert_eq!(game.awaiting(), Awaiting::Turn(PLAYER));
}

#[test]
fn initial_spawns_land_on_distinct_cells() {
    for seed in 0..200 {
        let game = Puzzle2048::new(&settings(2, 2, 8), &[PlayerSpec::HUMAN]).unwrap();
        let events = game.roll_world(&mut CoreRng::new(seed));
        assert_eq!(events.len(), 2);
        let [
            WorldEvent::TileSpawned { cell: first, .. },
            WorldEvent::TileSpawned { cell: second, .. },
        ] = events[..]
        else {
            panic!("two spawns expected");
        };
        assert_ne!(first, second);
    }
}

#[test]
fn one_spawn_per_move_and_none_otherwise() {
    let mut rng = CoreRng::new(5);
    let mut game = started(&Settings::default(), &mut rng);
    assert!(game.roll_world(&mut rng).is_empty());
    let direction = Direction::ALL
        .into_iter()
        .find(|&direction| game.can_move(direction))
        .unwrap();
    try_move(&mut game, direction).unwrap();
    let events = game.roll_world(&mut rng);
    assert_eq!(events.len(), 1);
    let WorldEvent::TileSpawned { cell, .. } = events[0];
    assert!(game.tile_at(cell).is_none());
}

#[test]
fn spawns_are_four_about_ten_percent_of_the_time() {
    let mut rng = CoreRng::new(99);
    let game = Puzzle2048::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    let mut fours = 0;
    for _ in 0..5_000 {
        for event in game.roll_world(&mut rng) {
            let WorldEvent::TileSpawned { value, .. } = event;
            if value == SpawnValue::Four {
                fours += 1;
            }
        }
    }
    assert!(
        (800..1200).contains(&fours),
        "{fours} fours in 10000 spawns"
    );
}

#[test]
fn moves_wait_for_the_pending_spawn() {
    let mut game = Puzzle2048::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    assert_eq!(
        try_move(&mut game, Direction::Left),
        Err(CommandError::Busy)
    );

    #[rustfmt::skip]
    let mut game = board(4, 4, &[
        2, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
    ]);
    try_move(&mut game, Direction::Right).unwrap();
    let before = game.state_hash();
    assert_eq!(
        try_move(&mut game, Direction::Left),
        Err(CommandError::Busy)
    );
    assert_eq!(game.state_hash(), before);
}

#[test]
fn only_the_player_seat_may_move() {
    let mut game = board(2, 2, &[2, 0, 0, 0]);
    assert_eq!(
        game.apply_command(Slot::new(1), &Command::Move(Direction::Right)),
        Err(CommandError::NotPlaying)
    );
}

#[test]
fn invalid_spawns_are_rejected_without_changes() {
    let mut game = board(2, 2, &[2, 0, 0, 0]);
    let before = game.state_hash();
    assert!(matches!(
        spawn(&mut game, 1, 1, SpawnValue::Two),
        Err(CommandError::Illegal(_))
    ));
    try_move(&mut game, Direction::Right).unwrap();
    let after_move = game.state_hash();
    assert_ne!(before, after_move);
    for (row, col) in [(0, 1), (2, 0), (0, 2), (u8::MAX, u8::MAX)] {
        assert!(matches!(
            spawn(&mut game, row, col, SpawnValue::Two),
            Err(CommandError::Illegal(_))
        ));
        assert_eq!(game.state_hash(), after_move, "({row}, {col})");
    }
    spawn(&mut game, 1, 0, SpawnValue::Four).unwrap();
    assert_eq!(values(&game), [0, 2, 4, 0]);
}

#[test]
fn win_is_decided_after_the_spawn_lands() {
    #[rustfmt::skip]
    let mut game = board(2, 2, &[
        1024, 1024,
        0,    0,
    ]);
    try_move(&mut game, Direction::Left).unwrap();
    assert_eq!(game.phase(), Phase::Playing);
    assert_eq!(game.roll_world(&mut CoreRng::new(3)).len(), 1);
    spawn(&mut game, 1, 1, SpawnValue::Two).unwrap();
    assert_eq!(values(&game), [2048, 0, 0, 2]);
    assert_eq!(
        game.status(),
        Status::Finished(mg_core::Outcome {
            kind: OutcomeKind::Victory,
            winners: Vec::new(),
            scores: vec![2048],
        })
    );
    assert_eq!(game.awaiting(), Awaiting::Nobody);
    assert!(game.roll_world(&mut CoreRng::new(3)).is_empty());
    assert_eq!(
        try_move(&mut game, Direction::Down),
        Err(CommandError::GameOver)
    );
}

#[test]
fn largest_target_on_the_largest_board() {
    let mut values = vec![0; 100];
    values[0] = 32_768;
    values[1] = 32_768;
    let mut game = Puzzle2048::with_values(&settings(10, 10, 65_536), &values);
    play(&mut game, Direction::Left, &mut CoreRng::new(8)).unwrap();
    assert_eq!(game.phase(), Phase::Won);
    assert_eq!(game.score().get(), 65_536);
    assert_eq!(
        view(&game).highest_tile.map(|tile| tile.get()),
        Some(65_536)
    );
}

#[test]
fn dropping_the_player_is_a_defeat() {
    let mut rng = CoreRng::new(11);
    let mut game = started(&Settings::default(), &mut rng);
    game.drop_player(Slot::new(1));
    assert_eq!(game.phase(), Phase::Playing, "unknown seats are ignored");
    game.drop_player(PLAYER);
    assert_eq!(game.phase(), Phase::Abandoned);
    assert!(matches!(game.status(), Status::Finished(o) if o.kind == OutcomeKind::Defeat));
    assert_eq!(game.awaiting(), Awaiting::Nobody);
    assert!(game.roll_world(&mut rng).is_empty());
    for direction in Direction::ALL {
        assert_eq!(try_move(&mut game, direction), Err(CommandError::GameOver));
    }
}

#[test]
fn full_board_without_merges_accepts_no_move() {
    #[rustfmt::skip]
    let mut game = board(2, 2, &[
        2, 4,
        4, 2,
    ]);
    for direction in Direction::ALL {
        assert!(!game.can_move(direction));
        assert!(matches!(
            try_move(&mut game, direction),
            Err(CommandError::Illegal(_))
        ));
    }
}

#[test]
fn view_lists_only_the_directions_that_change_the_board() {
    #[rustfmt::skip]
    let mut game = board(2, 2, &[
        2, 0,
        0, 0,
    ]);
    assert_eq!(
        view(&game).movable_directions,
        [Direction::Down, Direction::Right]
    );
    try_move(&mut game, Direction::Down).unwrap();
    assert!(
        view(&game).movable_directions.is_empty(),
        "no move is accepted while the spawn is due"
    );
}

#[test]
fn view_lists_no_direction_on_a_stuck_board() {
    #[rustfmt::skip]
    let game = board(2, 2, &[
        2, 4,
        4, 2,
    ]);
    assert!(view(&game).movable_directions.is_empty());
}

#[test]
fn can_move_predicts_acceptance() {
    let mut rng = CoreRng::new(21);
    let mut game = started(&settings(3, 3, 64), &mut rng);
    while game.phase() == Phase::Playing {
        for direction in Direction::ALL {
            let predicted = game.can_move(direction);
            let mut copy = game.clone();
            assert_eq!(try_move(&mut copy, direction).is_ok(), predicted);
        }
        let direction = Direction::ALL
            .into_iter()
            .find(|&direction| game.can_move(direction))
            .unwrap();
        try_move(&mut game, direction).unwrap();
        deliver_spawns(&mut game, &mut rng);
    }
    assert!(tile_count(&game) > 0);
}
