use std::cell::Cell;

use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{BotSeat, CoreRng, Game, HasBots, PlayerSpec, Slot, Viewer};

use crate::{BotKind, Command, FirstPlayerMode, GameEnd, Mark, Point, RawSettings, TicTacToe};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;
/// Keeps `run_all` fast in debug builds; strength is covered by the bot tests.
const CONFORMANCE_MINIMAX_BUDGET: u64 = 100_000;

thread_local! {
    /// `Conformance::bots_for` only sees seat kinds, so the bot kind per seat comes from here;
    /// every test runs on its own thread and sets its own plan.
    static BOT_PLAN: Cell<[BotKind; 2]> = const { Cell::new([BotKind::Minimax, BotKind::Random]) };
}

fn with_bot_plan(plan: [BotKind; 2], run: impl FnOnce()) {
    BOT_PLAN.with(|cell| cell.set(plan));
    run();
}

fn setup(
    size: (u32, u32),
    win_count: u32,
    first_player: FirstPlayerMode,
    players: [PlayerSpec; 2],
) -> Setup<TicTacToe> {
    Setup::new(
        RawSettings {
            field_width: size.0,
            field_height: size.1,
            win_count,
            first_player,
        },
        players.to_vec(),
    )
}

impl Conformance for TicTacToe {
    fn sample_setups() -> Vec<Setup<Self>> {
        use FirstPlayerMode::{Guest, Host, Random};
        vec![
            setup((3, 3), 3, Host, [HUMAN, HUMAN]),
            setup((3, 3), 3, Guest, [HUMAN, HUMAN]),
            setup((9, 9), 4, Guest, [HUMAN, BOT]),
            setup((3, 3), 3, Random, [HUMAN, HUMAN]),
            setup((15, 15), 5, Random, [HUMAN, HUMAN]),
            setup((20, 20), 5, Host, [HUMAN, HUMAN]),
            setup((4, 7), 4, Random, [HUMAN, HUMAN]),
            setup((20, 3), 3, Host, [HUMAN, HUMAN]),
            setup((3, 3), 3, Random, [HUMAN, BOT]),
            setup((3, 3), 3, Host, [BOT, HUMAN]),
            setup((15, 15), 5, Random, [HUMAN, BOT]),
            setup((15, 15), 5, Host, [BOT, HUMAN]),
            setup((7, 5), 4, Random, [BOT, HUMAN]),
            setup((3, 3), 3, Random, [BOT, BOT]),
            setup((9, 9), 4, Host, [BOT, BOT]),
            setup((15, 15), 5, Random, [BOT, BOT]),
            setup((20, 20), 5, Host, [BOT, BOT]),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        use FirstPlayerMode::{Host, Random};
        let mut setups: Vec<Setup<Self>> = [0, 2, 21, u32::MAX]
            .into_iter()
            .flat_map(|size| {
                [
                    setup((size, 15), 3, Random, [HUMAN, HUMAN]),
                    setup((15, size), 3, Host, [HUMAN, BOT]),
                    setup((size, size), 3, Random, [BOT, BOT]),
                ]
            })
            .collect();
        for win_count in [0, 2, 16, u32::MAX] {
            setups.push(setup((15, 15), win_count, Random, [HUMAN, HUMAN]));
        }
        setups.push(setup((4, 7), 5, Host, [HUMAN, HUMAN]));
        for players in [vec![HUMAN], vec![BOT], vec![HUMAN; 3], vec![], vec![BOT; 3]] {
            setups.push(Setup::new(RawSettings::default(), players));
        }
        setups
    }

    /// Mostly a random empty cell; sometimes any coordinate, on or off the board.
    fn arbitrary_command(game: &Self, _slot: Slot, rng: &mut CoreRng) -> Command {
        let board = game.board();
        let dimensions = board.dimensions();
        if rng.chance(7, 8) {
            let empty: Vec<Point> = dimensions
                .points()
                .filter(|point| board.mark_at(*point).is_none())
                .collect();
            if let Some(point) = rng.pick(&empty) {
                return Command::Place(*point);
            }
        }
        let coordinate = |rng: &mut CoreRng, limit: u32| match rng.below(10) {
            0 => u32::MAX,
            _ => rng.below(limit + 2),
        };
        let x = coordinate(rng, dimensions.width());
        let y = coordinate(rng, dimensions.height());
        Command::Place(Point::new(x, y))
    }

    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Self>> {
        let plan = BOT_PLAN.with(Cell::get);
        Slot::all(players.len())
            .zip(plan)
            .filter(|(slot, _)| {
                players
                    .get(slot.index())
                    .is_some_and(|spec| !spec.is_human())
            })
            .map(|(slot, kind)| BotSeat {
                slot,
                bot: TicTacToe::make_bot(kind),
                budget_nodes: match kind {
                    BotKind::Minimax => CONFORMANCE_MINIMAX_BUDGET,
                    BotKind::Random => TicTacToe::default_bot_budget(kind),
                },
            })
            .collect()
    }

    fn check_invariants(&self) -> Result<(), String> {
        let view = self.view(Viewer::Spectator);
        let count = |mark| {
            view.cells
                .iter()
                .filter(|cell| **cell == Some(mark))
                .count()
        };
        let (xs, os) = (count(Mark::X), count(Mark::O));
        if xs != os && xs != os + 1 {
            return Err(format!("{xs} X marks vs {os} O marks"));
        }
        if view.x_player.is_none() && xs + os > 0 {
            return Err("marks placed before the first player was chosen".into());
        }
        let lines: Vec<_> = (0..view.field_height)
            .flat_map(|y| (0..view.field_width).map(move |x| Point::new(x, y)))
            .filter_map(|point| self.board().line_through(point, self.win_length()))
            .collect();
        match view.end {
            None => {
                if let Some(line) = lines.first() {
                    return Err(format!("unfinished game has a line {line:?}"));
                }
                let mark_to_move = if xs == os { Mark::X } else { Mark::O };
                if view.x_player.is_some() && view.current_player != self.player_with(mark_to_move)
                {
                    return Err("wrong player to move".into());
                }
            }
            Some(GameEnd::Win { winner, line }) => {
                if self.mark_of(winner) != Some(line.mark) || !lines.contains(&line) {
                    return Err(format!("{winner} won with {line:?}, board lines {lines:?}"));
                }
                if lines.iter().any(|other| other.mark != line.mark) {
                    return Err("both marks have a line".into());
                }
                let last_mark = if xs > os { Mark::X } else { Mark::O };
                if last_mark != line.mark {
                    return Err("the winner did not make the last move".into());
                }
            }
            Some(GameEnd::Draw) => {
                if !lines.is_empty() || view.cells.iter().any(Option::is_none) {
                    return Err("draw with a line or an empty cell".into());
                }
            }
            Some(GameEnd::OpponentLeft { .. }) => {
                if !lines.is_empty() {
                    return Err("forfeit after a line was completed".into());
                }
            }
        }
        Ok(())
    }

    fn max_frames() -> u64 {
        // At most 400 moves plus the setup frame; anything longer means the game stalled.
        500
    }
}

#[test]
fn conformance_minimax_host_seat_random_guest_seat() {
    with_bot_plan([BotKind::Minimax, BotKind::Random], || {
        conformance::run_all::<TicTacToe>(6);
    });
}

#[test]
fn conformance_random_host_seat_minimax_guest_seat() {
    with_bot_plan([BotKind::Random, BotKind::Minimax], || {
        conformance::run_all::<TicTacToe>(4);
    });
}

#[test]
fn conformance_minimax_against_itself() {
    with_bot_plan([BotKind::Minimax, BotKind::Minimax], || {
        conformance::run_all::<TicTacToe>(2);
    });
}

#[test]
fn conformance_random_against_itself() {
    with_bot_plan([BotKind::Random, BotKind::Random], || {
        conformance::run_all::<TicTacToe>(4);
    });
}

/// Covers the rules, the random first-player event and the minimax bot (slot 0). Must be equal
/// on native and wasm32.
const GOLDEN_HASH: u64 = 0x46b1_c127_42ca_072a;

#[test]
fn scripted_hash_is_pinned() {
    let golden = setup((9, 9), 4, FirstPlayerMode::Random, [BOT, HUMAN]);
    with_bot_plan([BotKind::Minimax, BotKind::Random], || {
        assert_eq!(conformance::scripted_hash(&golden, 42), GOLDEN_HASH);
    });
}
