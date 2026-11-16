use std::fmt;

use mg_registry::GameKind;

use crate::seat_tally::SeatTally;

#[derive(Clone, Debug, PartialEq)]
pub struct ArenaReport {
    pub game: GameKind,
    pub games: u32,
    pub seed: u64,
    /// In slot order.
    pub seats: Vec<SeatTally>,
    /// Frames of all games together.
    pub total_ticks: u64,
    pub replays_written: u32,
}

impl ArenaReport {
    pub fn average_ticks(&self) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            self.total_ticks as f64 / f64::from(self.games)
        }
    }
}

impl fmt::Display for ArenaReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}: {} games, seed {}",
            self.game.info().id,
            self.games,
            self.seed
        )?;
        writeln!(
            f,
            "{:<5} {:<12} {:>6} {:>6} {:>7} {:>11}",
            "seat", "bot", "wins", "draws", "losses", "unfinished"
        )?;
        for seat in &self.seats {
            writeln!(
                f,
                "{:<5} {:<12} {:>6} {:>6} {:>7} {:>11}",
                seat.slot.to_string(),
                seat.bot.name,
                seat.wins,
                seat.draws,
                seat.losses,
                seat.unfinished
            )?;
        }
        writeln!(f, "average length: {:.1} ticks", self.average_ticks())?;
        if self.replays_written > 0 {
            writeln!(f, "replays written: {}", self.replays_written)?;
        }
        Ok(())
    }
}
