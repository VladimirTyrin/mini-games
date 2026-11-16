use super::side::Side;
use super::side_probe::SideProbe;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Probe {
    pub(crate) bot: SideProbe,
    pub(crate) opponent: SideProbe,
}

impl Probe {
    pub(crate) fn of(self, side: Side) -> SideProbe {
        match side {
            Side::Bot => self.bot,
            Side::Opponent => self.opponent,
        }
    }

    /// Change of the bot-perspective evaluation when `side` plays here.
    pub(crate) fn evaluation_delta(self, side: Side) -> i64 {
        match side {
            Side::Bot => self.bot.gain,
            Side::Opponent => self.opponent.gain.saturating_neg(),
        }
    }

    /// Move ordering key: attack value plus the value of denying the cell to the other side.
    pub(crate) fn priority(self, side: Side) -> i64 {
        self.of(side)
            .gain
            .saturating_add(self.of(side.other()).gain)
    }
}
