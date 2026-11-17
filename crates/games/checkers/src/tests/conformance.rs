use std::cell::Cell;

use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{BotSeat, CoreRng, Game, HasBots, PlayerSpec, Slot, Viewer};

use crate::{BotKind, Checkers, Command, GameEnd, Path, RawSettings, Side, Square, WhitePlayer};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;
/// Keeps `run_all` fast in debug builds; strength is covered by the bot tests.
const CONFORMANCE_SEARCH_BUDGET: u64 = 3_000;

thread_local! {
    /// `Conformance::bots_for` only sees seat kinds, so the bot kind per seat comes from here;
    /// every test runs on its own thread and sets its own plan.
    static BOT_PLAN: Cell<[BotKind; 2]> = const { Cell::new([BotKind::Hard, BotKind::Easy]) };
}

fn with_bot_plan(plan: [BotKind; 2], run: impl FnOnce()) {
    BOT_PLAN.with(|cell| cell.set(plan));
    run();
}

fn setup(white_player: WhitePlayer, players: [PlayerSpec; 2]) -> Setup<Checkers> {
    Setup::new(RawSettings { white_player }, players.to_vec())
}

impl Conformance for Checkers {
    fn sample_setups() -> Vec<Setup<Self>> {
        use WhitePlayer::{Guest, Host, Random};
        vec![
            setup(Random, [HUMAN, HUMAN]),
            setup(Host, [HUMAN, HUMAN]),
            setup(Guest, [HUMAN, HUMAN]),
            setup(Random, [HUMAN, BOT]),
            setup(Host, [BOT, HUMAN]),
            setup(Random, [BOT, BOT]),
            setup(Guest, [BOT, BOT]),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        [vec![HUMAN], vec![BOT], vec![HUMAN; 3], vec![], vec![BOT; 3]]
            .into_iter()
            .map(|players| Setup::new(RawSettings::default(), players))
            .collect()
    }

    /// Mostly a legal move; sometimes a random path (off the board, too long, too short).
    fn arbitrary_command(game: &Self, _slot: Slot, rng: &mut CoreRng) -> Command {
        if rng.chance(7, 8) {
            let moves = game.legal_moves();
            if let Some(chosen) = rng.pick(&moves) {
                return Command::Move(chosen.path.clone());
            }
        }
        let length = 2 + rng.below(4) as usize;
        let squares: Vec<Square> = (0..length)
            .filter_map(|_| Square::from_index(rng.below(70)))
            .collect();
        let fallback =
            || Path::new(vec![Square::new(0, 0).unwrap(), Square::new(2, 2).unwrap()]).unwrap();
        Command::Move(Path::new(squares).unwrap_or_else(fallback))
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
                bot: Checkers::make_bot(kind),
                budget_nodes: match kind {
                    BotKind::Easy => Checkers::default_bot_budget(kind),
                    BotKind::Medium | BotKind::Hard | BotKind::Master => CONFORMANCE_SEARCH_BUDGET,
                },
            })
            .collect()
    }

    fn check_invariants(&self) -> Result<(), String> {
        let view = self.view(Viewer::Spectator);
        let count = |side| {
            view.pieces
                .iter()
                .filter(|placed| placed.piece.side == side)
                .count()
        };
        let (white, black) = (count(Side::White), count(Side::Black));
        if white > 12 || black > 12 {
            return Err(format!("{white} white and {black} black pieces"));
        }
        let mut ids: Vec<u8> = view
            .pieces
            .iter()
            .map(|placed| placed.piece.id.get())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() != view.pieces.len() {
            return Err("two pieces share an id".into());
        }
        match view.end {
            None => {
                if view.white_player.is_some() && view.legal_moves.is_empty() {
                    return Err("the side to move has no move, yet the game goes on".into());
                }
            }
            Some(GameEnd::Win { winner, .. }) => {
                if self.side_of(winner).is_none() {
                    return Err(format!("{winner} won without a colour"));
                }
            }
            Some(GameEnd::Draw(_)) => {}
        }
        Ok(())
    }

    fn max_frames() -> u64 {
        // Every man move or capture is irreversible and at most 30 king moves follow each:
        // anything longer than this means the game does not end.
        4_000
    }
}

#[test]
fn conformance_hard_host_seat_easy_guest_seat() {
    with_bot_plan([BotKind::Hard, BotKind::Easy], || {
        conformance::run_all::<Checkers>(6);
    });
}

#[test]
fn conformance_easy_host_seat_medium_guest_seat() {
    with_bot_plan([BotKind::Easy, BotKind::Medium], || {
        conformance::run_all::<Checkers>(6);
    });
}

/// Pins the rules: if this value changes, a rule changed — update it deliberately.
const GOLDEN_HASH: u64 = 0x0f0e_ed20_64b3_1578;

#[test]
fn scripted_hash_is_pinned() {
    let setup = Setup::<Checkers>::new(RawSettings::default(), vec![HUMAN; 2]);
    let hash = conformance::scripted_hash(&setup, 42);
    assert_eq!(hash, GOLDEN_HASH, "pin GOLDEN_HASH = {hash:#018x}");
}
