mod bots;
mod conformance;
mod rules;
mod timing;

use mg_core::{Bot, Budget, CoreRng, Game, PlayerSpec, Slot};

use crate::{Command, FirstPlayerMode, Point, Settings, TicTacToe};

const HOST: Slot = Slot::new(0);
const GUEST: Slot = Slot::new(1);

fn settings(field_width: u32, field_height: u32, win_count: u32) -> Settings {
    Settings {
        field_width,
        field_height,
        win_count,
        first_player: FirstPlayerMode::Host,
    }
}

/// Host mode, two humans: slot 0 is X.
fn new_game(field_width: u32, field_height: u32, win_count: u32) -> TicTacToe {
    TicTacToe::new(
        &settings(field_width, field_height, win_count),
        &[PlayerSpec::HUMAN, PlayerSpec::HUMAN],
    )
    .unwrap()
}

fn place(game: &mut TicTacToe, x: u32, y: u32) {
    let slot = game.current_player().unwrap();
    game.apply_command(slot, &Command::Place(Point::new(x, y)))
        .unwrap_or_else(|error| panic!("({x}, {y}) by {slot}: {error}"));
}

/// Host-mode game with X on `xs` and O on `os`, placed alternately (X first), so
/// `xs.len()` must be `os.len()` or `os.len() + 1`.
fn position(size: u32, win_count: u32, xs: &[(u32, u32)], os: &[(u32, u32)]) -> TicTacToe {
    assert!(xs.len() == os.len() || xs.len() == os.len() + 1);
    let mut game = new_game(size, size, win_count);
    for (turn, &(x, y)) in xs.iter().enumerate() {
        place(&mut game, x, y);
        if let Some(&(x, y)) = os.get(turn) {
            place(&mut game, x, y);
        }
    }
    game
}

fn decide(bot: &mut dyn Bot<TicTacToe>, game: &TicTacToe, nodes: u64) -> Point {
    let me = game.current_player().unwrap();
    let mut rng = CoreRng::new(7);
    let Some(Command::Place(point)) = bot.decide(game, me, &mut rng, &mut Budget::new(nodes))
    else {
        panic!("bot returned no move");
    };
    point
}
