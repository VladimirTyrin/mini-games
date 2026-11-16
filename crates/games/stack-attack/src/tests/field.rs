use super::support::pattern;
use crate::field::Field;
use crate::{BoxId, CargoBox, Cell, Row};

fn cargo(id: BoxId, x: i32, y: i32) -> CargoBox {
    CargoBox {
        id,
        cell: Cell::new(x, y),
        pattern: pattern(),
        falling: false,
    }
}

fn ids() -> impl Iterator<Item = BoxId> {
    std::iter::successors(Some(BoxId::FIRST), |id| Some(id.next()))
}

#[test]
fn boxes_never_overwrite_each_other() {
    let mut field = Field::new(6, 6);
    let mut ids = ids();
    let (first, second) = (ids.next().unwrap(), ids.next().unwrap());
    assert!(field.try_add(cargo(first, 1, 1)));
    assert!(!field.try_add(cargo(second, 1, 1)), "ghost box");
    assert!(!field.try_add(cargo(first, 2, 2)), "duplicate id");
    assert!(!field.try_add(cargo(second, 6, 0)), "off field");
    assert!(field.try_add(cargo(second, 2, 1)));
    assert!(!field.try_move(second, Cell::new(1, 1)));
    assert_eq!(field.boxes().len(), 2);
    field.check_consistency().unwrap();
}

/// The legacy state A9 left behind: a resting box ends up above a hole after the shift.
#[test]
fn line_clear_makes_boxes_above_a_hole_fall() {
    let mut field = Field::new(6, 6);
    let mut ids = ids();
    for x in 0..6 {
        if x != 2 {
            assert!(field.try_add(cargo(ids.next().unwrap(), x, 0)));
        }
        assert!(field.try_add(cargo(ids.next().unwrap(), x, 1)));
    }
    let top = ids.next().unwrap();
    assert!(field.try_add(cargo(top, 2, 2)));

    assert_eq!(field.clear_full_rows(), vec![Row::new(1)]);
    let shifted = field.boxes().iter().find(|b| b.id == top).copied().unwrap();
    assert_eq!(shifted.cell, Cell::new(2, 1));
    assert!(shifted.falling);

    field.apply_gravity();
    assert_eq!(field.box_at(Cell::new(2, 0)), Some(top));
    field.check_consistency().unwrap();
}
