use mg_core::{
    Authority, BotSeat, CommandError, CoreRng, Game, HasBots, OutcomeKind, PlayerSpec, Slot,
    Status, Viewer,
};

use crate::{
    BotKind, Cell, CellMark, Command, Fleet, FleetError, Orientation, Phase, SeaBattle,
    SecretCommand, Settings, Ship, ShotOutcome, WorldEvent,
};

const FIRST: Slot = Slot::new(0);
const SECOND: Slot = Slot::new(1);
use Orientation::{Horizontal, Vertical};

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn ship(x: u8, y: u8, length: u8, orientation: Orientation) -> Ship {
    Ship::new(cell(x, y), length, orientation).unwrap()
}

/// Rows 0, 2 and 4, a cell of water between ships.
fn classic_ships() -> Vec<Ship> {
    vec![
        ship(0, 0, 4, Horizontal),
        ship(5, 0, 3, Horizontal),
        ship(0, 2, 3, Horizontal),
        ship(4, 2, 2, Horizontal),
        ship(7, 2, 2, Horizontal),
        ship(0, 4, 2, Horizontal),
        ship(3, 4, 1, Horizontal),
        ship(5, 4, 1, Horizontal),
        ship(7, 4, 1, Horizontal),
        ship(9, 4, 1, Horizontal),
    ]
}

fn place(ships: Vec<Ship>) -> SecretCommand {
    SecretCommand::PlaceFleet(ships)
}

fn authority(players: &[PlayerSpec], bots: Vec<BotSeat<SeaBattle>>) -> Authority<SeaBattle> {
    let mut authority = Authority::<SeaBattle>::new(&Settings, players, 7, bots).unwrap();
    authority.seal_frame();
    authority
}

/// Two people, both fleets placed as [`classic_ships`]: the battle is on.
fn battle() -> Authority<SeaBattle> {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    for slot in [FIRST, SECOND] {
        authority
            .submit_secret(slot, &place(classic_ships()))
            .unwrap();
    }
    authority.seal_frame();
    authority
}

fn shooter(authority: &Authority<SeaBattle>) -> Slot {
    authority.game().view(Viewer::Spectator).turn.unwrap()
}

fn fire(authority: &mut Authority<SeaBattle>, at: Cell) -> Result<(), CommandError> {
    let by = shooter(authority);
    authority.submit(by, Command::Fire(at))?;
    authority.seal_frame();
    Ok(())
}

fn other(slot: Slot) -> Slot {
    if slot == FIRST { SECOND } else { FIRST }
}

fn mark_at(authority: &Authority<SeaBattle>, seat: Slot, at: Cell) -> CellMark {
    authority.game().waters(seat).unwrap().mark(at)
}

#[test]
fn a_fleet_is_the_classic_ships_none_touching() {
    assert!(Fleet::parse(&classic_ships()).is_ok());
    let mut short = classic_ships();
    short.pop();
    assert_eq!(Fleet::parse(&short), Err(FleetError::WrongShips));
    let mut extra = classic_ships();
    extra.push(ship(9, 9, 1, Horizontal));
    assert_eq!(Fleet::parse(&extra), Err(FleetError::WrongShips));
    // The single at (3, 4) moved to touch the corner of the three at (0..=2, 2).
    let mut corner = classic_ships();
    corner[6] = ship(3, 3, 1, Horizontal);
    assert_eq!(Fleet::parse(&corner), Err(FleetError::ShipsTouch));
}

#[test]
fn a_ship_stays_on_the_board() {
    assert!(Ship::new(cell(7, 0), 4, Horizontal).is_none());
    assert!(Ship::new(cell(6, 0), 4, Horizontal).is_some());
    assert!(Ship::new(cell(0, 9), 2, Vertical).is_none());
    assert!(Ship::new(cell(0, 0), 5, Vertical).is_none());
    assert!(Ship::new(cell(0, 0), 0, Vertical).is_none());
}

#[test]
fn random_fleets_are_legal_and_differ() {
    let mut rng = CoreRng::new(3);
    let fleets: Vec<Fleet> = (0..50).map(|_| Fleet::random(&mut rng).unwrap()).collect();
    for fleet in &fleets {
        assert!(Fleet::parse(fleet.ships()).is_ok());
    }
    assert_ne!(fleets[0], fleets[1]);
}

#[test]
fn the_battle_starts_once_both_fleets_are_placed() {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    assert!(authority.game().awaits_secret(FIRST));
    authority
        .submit_secret(FIRST, &place(classic_ships()))
        .unwrap();
    authority.seal_frame();
    let view = authority.game().view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Placing);
    assert_eq!(view.ready, [true, false]);
    assert_eq!(
        authority.submit(FIRST, Command::Fire(cell(0, 0))),
        Err(CommandError::Illegal("the battle has not started yet"))
    );

    authority
        .submit_secret(SECOND, &place(classic_ships()))
        .unwrap();
    authority.seal_frame();
    let view = authority.game().view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Battle);
    assert!(view.turn.is_some(), "someone fires first");
}

#[test]
fn a_fleet_is_placed_once_and_must_be_legal() {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    let mut touching = classic_ships();
    touching[6] = ship(3, 3, 1, Horizontal);
    assert_eq!(
        authority.submit_secret(FIRST, &place(touching)),
        Err(CommandError::Illegal(FleetError::ShipsTouch.reason()))
    );
    authority
        .submit_secret(FIRST, &place(classic_ships()))
        .unwrap();
    let placed = authority.seat_secret(FIRST);
    let mut other = classic_ships();
    other[9] = ship(9, 9, 1, Horizontal);
    assert!(authority.submit_secret(FIRST, &place(other)).is_err());
    assert_eq!(
        authority.seat_secret(FIRST),
        placed,
        "a refused fleet changes nothing"
    );
    assert_eq!(authority.seat_secret(SECOND), None);
}

#[test]
fn each_seat_sees_only_its_own_fleet() {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    authority
        .submit_secret(FIRST, &place(classic_ships()))
        .unwrap();
    assert_eq!(
        authority.seat_secret(FIRST),
        Some(Fleet::parse(&classic_ships()).unwrap())
    );
    assert_eq!(authority.seat_secret(SECOND), None);
}

#[test]
fn a_seat_out_of_time_gets_a_random_fleet() {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    authority.submit_default_secret(FIRST).unwrap();
    assert!(
        authority.submit_default_secret(FIRST).is_err(),
        "placed already"
    );
    authority.seal_frame();
    assert_eq!(
        authority.game().view(Viewer::Spectator).ready,
        [true, false]
    );
    assert!(authority.seat_secret(FIRST).is_some());
}

#[test]
fn a_hit_fires_again_and_a_miss_passes_the_turn() {
    let mut authority = battle();
    let first = shooter(&authority);
    fire(&mut authority, cell(0, 0)).unwrap();
    assert_eq!(mark_at(&authority, other(first), cell(0, 0)), CellMark::Hit);
    assert_eq!(shooter(&authority), first);

    fire(&mut authority, cell(9, 9)).unwrap();
    assert_eq!(
        mark_at(&authority, other(first), cell(9, 9)),
        CellMark::Miss
    );
    assert_eq!(shooter(&authority), other(first));
    let last = authority.game().view(Viewer::Spectator).last_shot.unwrap();
    assert_eq!(
        (last.by, last.cell, last.outcome),
        (first, cell(9, 9), ShotOutcome::Miss)
    );
}

#[test]
fn a_known_cell_and_a_shot_out_of_turn_are_refused() {
    let mut authority = battle();
    let first = shooter(&authority);
    assert_eq!(
        authority.submit(other(first), Command::Fire(cell(5, 5))),
        Err(CommandError::NotYourTurn)
    );
    fire(&mut authority, cell(0, 0)).unwrap();
    assert_eq!(
        fire(&mut authority, cell(0, 0)),
        Err(CommandError::Illegal("that cell is known already"))
    );
}

#[test]
fn sinking_a_ship_clears_the_water_around_it() {
    let mut authority = battle();
    let first = shooter(&authority);
    for x in 0..4 {
        fire(&mut authority, cell(x, 0)).unwrap();
    }
    let target = other(first);
    let waters = authority.game().waters(target).unwrap();
    assert_eq!(waters.sunk(), [ship(0, 0, 4, Horizontal)]);
    assert_eq!(waters.mark(cell(2, 0)), CellMark::Sunk);
    assert_eq!(waters.mark(cell(4, 0)), CellMark::Cleared);
    assert_eq!(waters.mark(cell(4, 1)), CellMark::Cleared);
    assert_eq!(waters.afloat(), [3, 3, 2, 2, 2, 1, 1, 1, 1]);
    assert_eq!(
        fire(&mut authority, cell(4, 1)),
        Err(CommandError::Illegal("that cell is known already"))
    );
}

#[test]
fn sinking_the_whole_fleet_wins_and_reveals_both_fleets() {
    let mut authority = battle();
    let first = shooter(&authority);
    for target in classic_ships().iter().flat_map(|ship| ship.cells()) {
        fire(&mut authority, target).unwrap();
    }
    let view = authority.game().view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Won);
    assert_eq!(view.winner, Some(first));
    let Status::Finished(outcome) = authority.game().status() else {
        panic!("not finished");
    };
    assert_eq!(outcome.kind, OutcomeKind::Winner);
    assert_eq!(outcome.winners, [first]);
    let mut expected = classic_ships();
    expected.sort_unstable();
    assert_eq!(view.revealed, [Some(expected.clone()), Some(expected)]);
}

/// The fleets reach the players only at the end: no frame before it carries one, and the
/// game everyone replays holds none.
#[test]
fn the_fleets_are_sent_only_once_the_game_is_over() {
    let mut authority = battle();
    fire(&mut authority, cell(9, 9)).unwrap();
    assert!(!format!("{:?}", authority.game()).contains("Fleet {"));
    let loser = shooter(&authority);
    authority.submit(loser, Command::GiveUp).unwrap();
    authority.seal_frame();

    let (last, earlier) = authority.compact_log().split_last().unwrap();
    for frame in earlier {
        let sent = format!("{frame:?}");
        assert!(!sent.contains("FleetRevealed"), "{sent}");
        assert!(!sent.contains("PlaceFleet"), "{sent}");
    }
    let revealed = last
        .inputs
        .iter()
        .filter(|input| {
            matches!(
                input,
                mg_core::Input::World(WorldEvent::FleetRevealed { .. })
            )
        })
        .count();
    assert_eq!(revealed, 2);
}

#[test]
fn giving_up_hands_the_win_to_the_other_seat() {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    authority.submit(SECOND, Command::GiveUp).unwrap();
    authority.seal_frame();
    let view = authority.game().view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Forfeited);
    assert_eq!(view.winner, Some(FIRST));
}

#[test]
fn leaving_while_placing_hands_the_win_to_the_other_seat() {
    let mut authority = authority(&[PlayerSpec::HUMAN; 2], Vec::new());
    authority
        .submit_secret(FIRST, &place(classic_ships()))
        .unwrap();
    authority.drop_player(FIRST);
    authority.seal_frame();
    let view = authority.game().view(Viewer::Spectator);
    assert_eq!(view.winner, Some(SECOND));
    // Only the fleet that was placed is shown.
    assert!(view.revealed[0].is_some());
    assert_eq!(view.revealed[1], None);
}

fn bot_seat(slot: Slot, kind: BotKind) -> BotSeat<SeaBattle> {
    BotSeat {
        slot,
        bot: SeaBattle::make_bot(kind),
        budget_nodes: SeaBattle::default_bot_budget(kind),
    }
}

/// Plays two bots to the end; the winner's slot.
fn bot_match(first: BotKind, second: BotKind, seed: u64) -> Slot {
    let bots = vec![bot_seat(FIRST, first), bot_seat(SECOND, second)];
    let mut authority =
        Authority::<SeaBattle>::new(&Settings, &[PlayerSpec::BOT; 2], seed, bots).unwrap();
    for _ in 0..250 {
        authority.run_bots();
        authority.seal_frame();
        if authority.game().status().is_finished() {
            break;
        }
    }
    assert_eq!(authority.rejected_bot_commands(), 0);
    let view = authority.game().view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Won, "{first:?} against {second:?}");
    view.winner.unwrap()
}

#[test]
fn every_bot_places_a_fleet_and_fires_legal_shots_to_the_end() {
    for first in [BotKind::Easy, BotKind::Medium, BotKind::Hard] {
        for second in [BotKind::Easy, BotKind::Medium, BotKind::Hard] {
            bot_match(first, second, 5);
        }
    }
}

#[test]
fn a_stronger_bot_beats_a_weaker_one_most_of_the_time() {
    let wins = |strong, weak| {
        (0..20)
            .filter(|seed| {
                // Seats alternate, so who fires first does not decide it.
                if seed % 2 == 0 {
                    bot_match(strong, weak, *seed) == FIRST
                } else {
                    bot_match(weak, strong, *seed) == SECOND
                }
            })
            .count()
    };
    assert!(wins(BotKind::Hard, BotKind::Easy) >= 16);
    assert!(wins(BotKind::Medium, BotKind::Easy) >= 14);
    assert!(wins(BotKind::Hard, BotKind::Medium) >= 11);
}
