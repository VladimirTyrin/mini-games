use mg_core::CoreRng;

use super::support::{board, play, settings, spawn, started, try_move, view};
use crate::{
    Cell, Direction, MergeSource, Phase, SpawnValue, Tile, TileId, TileOrigin, TileValue, View,
};

fn tile(view: &View, row: u8, col: u8) -> Tile {
    *view
        .tile_at(Cell::new(row, col))
        .unwrap_or_else(|| panic!("no tile at ({row}, {col})"))
}

fn id(game_view: &View, row: u8, col: u8) -> TileId {
    tile(game_view, row, col).id
}

fn value(raw: u32) -> TileValue {
    TileValue::new(raw).unwrap()
}

#[test]
fn scripted_moves_report_slides_merges_and_spawns() {
    #[rustfmt::skip]
    let mut game = board(4, 4, &[
        2, 2, 4, 0,
        0, 0, 0, 4,
        0, 0, 0, 0,
        0, 0, 0, 0,
    ]);
    let before = view(&game);
    let (a, b, c, d) = (
        id(&before, 0, 0),
        id(&before, 0, 1),
        id(&before, 0, 2),
        id(&before, 1, 3),
    );

    try_move(&mut game, Direction::Left).unwrap();
    spawn(&mut game, 3, 3, SpawnValue::Two).unwrap();
    let after_left = view(&game);
    assert_eq!(after_left.last_move, Some(Direction::Left));

    let merged = tile(&after_left, 0, 0);
    assert_eq!(merged.value, value(4));
    assert!(
        ![a, b, c, d].contains(&merged.id),
        "a merge creates a new tile"
    );
    assert_eq!(
        merged.origin,
        TileOrigin::Merged {
            sources: [
                MergeSource {
                    id: a,
                    from: Cell::new(0, 0)
                },
                MergeSource {
                    id: b,
                    from: Cell::new(0, 1)
                },
            ]
        }
    );
    assert_eq!(
        tile(&after_left, 0, 1),
        Tile {
            id: c,
            value: value(4),
            cell: Cell::new(0, 1),
            origin: TileOrigin::Slid {
                from: Cell::new(0, 2)
            },
        }
    );
    assert_eq!(
        tile(&after_left, 1, 0).origin,
        TileOrigin::Slid {
            from: Cell::new(1, 3)
        }
    );
    assert_eq!(id(&after_left, 1, 0), d);
    let spawned = tile(&after_left, 3, 3);
    assert_eq!(spawned.origin, TileOrigin::Spawned);
    assert_eq!(after_left.tiles.len(), 4);

    try_move(&mut game, Direction::Up).unwrap();
    spawn(&mut game, 3, 0, SpawnValue::Four).unwrap();
    let after_up = view(&game);
    assert_eq!(
        tile(&after_up, 0, 0).origin,
        TileOrigin::Merged {
            sources: [
                MergeSource {
                    id: merged.id,
                    from: Cell::new(0, 0)
                },
                MergeSource {
                    id: d,
                    from: Cell::new(1, 0)
                },
            ]
        }
    );
    assert_eq!(
        tile(&after_up, 0, 1).origin,
        TileOrigin::Slid {
            from: Cell::new(0, 1)
        },
        "a tile that stays put still reports where it was"
    );
    assert_eq!(
        tile(&after_up, 0, 3),
        Tile {
            id: spawned.id,
            value: value(2),
            cell: Cell::new(0, 3),
            origin: TileOrigin::Slid {
                from: Cell::new(3, 3)
            },
        }
    );
    assert_eq!(tile(&after_up, 3, 0).origin, TileOrigin::Spawned);
}

#[test]
fn initial_tiles_are_reported_as_spawned() {
    let game = started(&settings(4, 4, 2048), &mut CoreRng::new(4));
    let view = view(&game);
    assert_eq!(view.tiles.len(), 2);
    assert!(
        view.tiles
            .iter()
            .all(|tile| tile.origin == TileOrigin::Spawned)
    );
    assert_ne!(view.tiles[0].id, view.tiles[1].id);
    assert_eq!(view.last_move, None);
}

/// Everything a client needs to animate `before` → `after` is consistent with `before`.
fn check_transition(before: &View, after: &View) {
    assert_eq!(after.moves, before.moves + 1);
    let previous = |id: TileId| before.tiles.iter().find(|tile| tile.id == id);
    let mut accounted: Vec<TileId> = Vec::new();
    let mut spawned = 0;
    for tile in &after.tiles {
        match tile.origin {
            TileOrigin::Slid { from } => {
                let old = previous(tile.id).expect("slid tile existed before");
                assert_eq!((old.cell, old.value), (from, tile.value));
                accounted.push(tile.id);
            }
            TileOrigin::Merged { sources } => {
                assert!(previous(tile.id).is_none(), "merged tile has a fresh id");
                for source in sources {
                    let old = previous(source.id).expect("merge source existed before");
                    assert_eq!(old.cell, source.from);
                    assert_eq!(old.value.get() * 2, tile.value.get());
                    accounted.push(source.id);
                }
            }
            TileOrigin::Spawned => {
                assert!(previous(tile.id).is_none(), "spawned tile has a fresh id");
                assert!(tile.value == value(2) || tile.value == value(4));
                spawned += 1;
            }
        }
    }
    assert_eq!(spawned, 1);
    let mut ids: Vec<TileId> = after.tiles.iter().map(|tile| tile.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), after.tiles.len(), "ids are unique");
    accounted.sort();
    let mut old_ids: Vec<TileId> = before.tiles.iter().map(|tile| tile.id).collect();
    old_ids.sort();
    assert_eq!(
        accounted, old_ids,
        "every old tile either slid or was merged exactly once"
    );
}

#[test]
fn animation_data_is_consistent_across_random_games() {
    for seed in 0..30 {
        let mut rng = CoreRng::new(seed);
        let mut game = started(&settings(4, 4, 256), &mut rng);
        for _ in 0..300 {
            if game.phase() != Phase::Playing {
                break;
            }
            let direction = Direction::ALL[rng.below(4) as usize];
            let before = view(&game);
            if play(&mut game, direction, &mut rng).is_ok() {
                check_transition(&before, &view(&game));
            } else {
                assert_eq!(view(&game), before);
            }
        }
    }
}
