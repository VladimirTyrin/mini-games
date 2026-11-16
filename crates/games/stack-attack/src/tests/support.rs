use mg_core::{Game, GameSettings, PlayerSpec, Slot, Viewer};

use crate::{
    BoxId, CargoBox, Cell, Column, Command, Crane, CraneId, CraneState, PatternId, RawSettings,
    Settings, StackAttack, View, WorkerView,
};

pub(super) fn new_game(field_width: u32, field_height: u32, seats: usize) -> StackAttack {
    let settings = Settings::parse(RawSettings {
        field_width,
        field_height,
    })
    .unwrap();
    StackAttack::new(&settings, &vec![PlayerSpec::HUMAN; seats]).unwrap()
}

pub(super) fn slot(seat: u8) -> Slot {
    Slot::new(seat)
}

pub(super) fn pattern() -> PatternId {
    PatternId::new(0).unwrap()
}

fn put(game: &mut StackAttack, x: i32, y: i32, falling: bool) -> BoxId {
    let id = game.next_box_id;
    let added = game.field.try_add(CargoBox {
        id,
        cell: Cell::new(x, y),
        pattern: pattern(),
        falling,
    });
    assert!(added, "cell ({x}, {y}) is taken");
    game.next_box_id = id.next();
    id
}

pub(super) fn put_box(game: &mut StackAttack, x: i32, y: i32) -> BoxId {
    put(game, x, y, false)
}

pub(super) fn put_falling_box(game: &mut StackAttack, x: i32, y: i32) -> BoxId {
    put(game, x, y, true)
}

pub(super) fn fill_row(game: &mut StackAttack, y: i32, except: &[i32]) {
    for x in 0..game.field.width() {
        if !except.contains(&x) {
            put_box(game, x, y);
        }
    }
}

pub(super) fn place_worker(game: &mut StackAttack, seat: usize, x: i32, y: i32) {
    game.workers[seat].feet = Cell::new(x, y);
}

pub(super) fn add_crane(game: &mut StackAttack, x: i32, target_x: i32) -> CraneId {
    let id = game.next_crane_id;
    game.cranes.push(Crane {
        id,
        x: Column::new(x),
        target_x: Column::new(target_x),
        state: CraneState::Moving,
        pattern: pattern(),
    });
    game.next_crane_id = id.next();
    id
}

pub(super) fn send(game: &mut StackAttack, seat: u8, command: Command) {
    game.apply_command(slot(seat), &command).unwrap();
}

pub(super) fn steps(game: &mut StackAttack, count: usize) {
    for _ in 0..count {
        game.step();
    }
}

pub(super) fn view(game: &StackAttack) -> View {
    game.view(Viewer::Spectator)
}

pub(super) fn worker(game: &StackAttack, seat: usize) -> WorkerView {
    view(game).workers[seat]
}

pub(super) fn box_cells(game: &StackAttack) -> Vec<Cell> {
    view(game).boxes.iter().map(|b| b.cell).collect()
}

pub(super) fn box_cell(game: &StackAttack, id: BoxId) -> Option<Cell> {
    view(game).boxes.iter().find(|b| b.id == id).map(|b| b.cell)
}
