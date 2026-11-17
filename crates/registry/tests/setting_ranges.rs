use mg_core::{PlayerSpec, SettingRange, SettingsError};
use mg_protocol::proto;
use mg_registry::{CreateError, GameKind, create_mirror, default_settings, setting_ranges};

/// Writes `value` into the wire field the game reports as `field`. Goes through protobuf JSON,
/// which addresses fields by name, so every game (a scaffolded one too) is covered without a
/// per-game list; an unknown name fails to deserialize.
fn set_field(settings: &mut proto::GameSettings, field: &str, value: u32) {
    let mut json = serde_json::to_value(*settings).unwrap();
    let game = json.as_object_mut().unwrap().values_mut().next().unwrap();
    game.as_object_mut()
        .unwrap()
        .insert(json_name(field), value.into());
    *settings = serde_json::from_value(json)
        .unwrap_or_else(|error| panic!("no wire field `{field}`: {error}"));
}

/// Protobuf JSON's lowerCamelCase name of a proto field.
fn json_name(field: &str) -> String {
    let mut parts = field.split('_');
    let mut name = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    name
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
        // Checkers has no numeric setting (only who plays white) and Sea Battle no setting at
        // all; every other game has some, and must publish them.
        assert!(
            !ranges.is_empty() || matches!(kind, GameKind::Checkers | GameKind::SeaBattle),
            "{kind:?} publishes no ranges"
        );
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
