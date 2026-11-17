use mg_core::{Budget, CoreRng, Game, HasBots, PlayerSpec, Slot};

use crate::{BotKind, Checkers, GameEnd, Settings, WhitePlayer};

const HOST: Slot = Slot::new(0);
/// Small enough for debug builds; the strength gap shows already.
const TEST_SEARCH_BUDGET: u64 = 4_000;

/// Plays a whole game between two bot kinds (the host plays white); returns how it ended.
fn play(white: BotKind, black: BotKind, seed: u64) -> GameEnd {
    let mut game = Checkers::new(
        &Settings {
            white_player: WhitePlayer::Host,
        },
        &[PlayerSpec::BOT; 2],
    )
    .unwrap();
    let mut bots = [Checkers::make_bot(white), Checkers::make_bot(black)];
    let mut rng = CoreRng::new(seed);
    for _ in 0..2_000 {
        let Some(slot) = game.current_player() else {
            break;
        };
        let kind = if slot == HOST { white } else { black };
        let budget = match kind {
            BotKind::Easy => Checkers::default_bot_budget(kind),
            BotKind::Medium | BotKind::Hard | BotKind::Master => TEST_SEARCH_BUDGET,
        };
        let command = bots[slot.index()]
            .decide(&game, slot, &mut rng, &mut Budget::new(budget))
            .expect("a bot always has a move on its turn");
        game.apply_command(slot, &command)
            .expect("a bot only makes legal moves");
    }
    game.end().expect("the game ends")
}

#[test]
fn every_bot_plays_legal_moves_to_the_end() {
    for kind in [
        BotKind::Easy,
        BotKind::Medium,
        BotKind::Hard,
        BotKind::Master,
    ] {
        play(kind, BotKind::Easy, 7);
    }
}

#[test]
fn the_hard_bot_beats_the_easy_one() {
    let hard_wins = (0..4)
        .filter(|seed| {
            let white = play(BotKind::Hard, BotKind::Easy, *seed).winner() == Some(HOST);
            let black = play(BotKind::Easy, BotKind::Hard, *seed).winner() == Some(Slot::new(1));
            white && black
        })
        .count();

    assert!(
        hard_wins >= 3,
        "the hard bot won both colours in {hard_wins} of 4"
    );
}
