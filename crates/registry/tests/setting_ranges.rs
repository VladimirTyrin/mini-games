use mg_core::{PlayerSpec, SettingRange, SettingsError};
use mg_protocol::proto;
use mg_protocol::proto::game_settings::Game as WireSettings;
use mg_registry::{CreateError, GameKind, create_mirror, default_settings, setting_ranges};

/// Writes `value` into the wire field the game reports as `field`.
fn set_field(settings: &mut proto::GameSettings, field: &str, value: u32) {
    let slot = match (settings.game.as_mut().unwrap(), field) {
        (WireSettings::Snake(snake), "field_width") => &mut snake.field_width,
        (WireSettings::Snake(snake), "field_height") => &mut snake.field_height,
        (WireSettings::Snake(snake), "max_food_count") => &mut snake.max_food_count,
        (WireSettings::Snake(snake), "food_spawn_permille") => &mut snake.food_spawn_permille,
        (WireSettings::Snake(snake), "tick_interval_ms") => &mut snake.tick_interval_ms,
        (WireSettings::Tictactoe(tictactoe), "field_width") => &mut tictactoe.field_width,
        (WireSettings::Tictactoe(tictactoe), "field_height") => &mut tictactoe.field_height,
        (WireSettings::Tictactoe(tictactoe), "win_count") => &mut tictactoe.win_count,
        (WireSettings::StackAttack(stack), "field_width") => &mut stack.field_width,
        (WireSettings::StackAttack(stack), "field_height") => &mut stack.field_height,
        (WireSettings::Puzzle2048(puzzle), "width") => &mut puzzle.width,
        (WireSettings::Puzzle2048(puzzle), "height") => &mut puzzle.height,
        (WireSettings::Puzzle2048(puzzle), "target") => &mut puzzle.target,
        (WireSettings::NumbersMatch(numbers), "refills") => &mut numbers.refills,
        (game, field) => panic!("no wire field `{field}` in {game:?}"),
    };
    *slot = value;
}

fn rejected_field(kind: GameKind, settings: &proto::GameSettings) -> Option<&'static str> {
    let players = vec![PlayerSpec::HUMAN; usize::from(kind.info().min_players)];
    match create_mirror(settings, &players) {
        Ok(_) => None,
        Err(CreateError::Settings(SettingsError { field, .. })) => Some(field),
        Err(other) => panic!("unexpected setup error: {other}"),
    }
}

fn with_value(base: &proto::GameSettings, range: SettingRange, value: u32) -> proto::GameSettings {
    let mut settings = *base;
    set_field(&mut settings, range.field, value);
    settings
}

#[test]
fn published_ranges_are_the_ones_validation_enforces() {
    for &kind in GameKind::ALL {
        let defaults = default_settings(kind);
        let ranges = setting_ranges(&defaults).unwrap();
        assert!(!ranges.is_empty(), "{kind:?} publishes no ranges");
        for range in ranges {
            assert!(range.min <= range.max, "{kind:?} {range:?}");
            for inside in [range.min, range.max] {
                let settings = with_value(&defaults, range, inside);
                assert_ne!(
                    rejected_field(kind, &settings),
                    Some(range.field),
                    "{kind:?} {inside}"
                );
            }
            if let Some(below) = range.min.checked_sub(1) {
                let settings = with_value(&defaults, range, below);
                assert_eq!(
                    rejected_field(kind, &settings),
                    Some(range.field),
                    "{kind:?} {below}"
                );
            }
            let settings = with_value(&defaults, range, range.max + 1);
            assert_eq!(
                rejected_field(kind, &settings),
                Some(range.field),
                "{kind:?} {range:?}"
            );
        }
    }
}

#[test]
fn a_dependent_bound_follows_the_other_settings() {
    let mut settings = default_settings(GameKind::TicTacToe);
    set_field(&mut settings, "field_width", 9);
    set_field(&mut settings, "field_height", 12);
    let win_count = setting_ranges(&settings)
        .unwrap()
        .into_iter()
        .find(|range| range.field == "win_count")
        .unwrap();
    assert_eq!(win_count.max, 9);
}
