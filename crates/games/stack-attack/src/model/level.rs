use crate::model::TickCount;
use crate::rules::{
    CRANE_SPAWN_INTERVAL_STEP, INITIAL_CRANE_SPAWN_INTERVAL_TICKS, INITIAL_MAX_CRANES,
    LEVELS_PER_EXTRA_CRANE, MAX_CRANES_CAP, MIN_CRANE_SPAWN_INTERVAL_TICKS, TICKS_PER_LEVEL,
};

/// Difficulty, starting at 1 and growing with survival time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(u32);

impl Level {
    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn after_ticks(ticks: TickCount) -> Self {
        let raised = u32::try_from(ticks.get() / TICKS_PER_LEVEL).unwrap_or(u32::MAX);
        Self(raised.saturating_add(1))
    }

    pub fn crane_spawn_interval(self) -> u32 {
        let reduction = self
            .0
            .saturating_sub(1)
            .saturating_mul(CRANE_SPAWN_INTERVAL_STEP);
        INITIAL_CRANE_SPAWN_INTERVAL_TICKS
            .saturating_sub(reduction)
            .max(MIN_CRANE_SPAWN_INTERVAL_TICKS)
    }

    pub fn max_cranes(self) -> u32 {
        let extra = self.0.saturating_sub(1) / LEVELS_PER_EXTRA_CRANE;
        INITIAL_MAX_CRANES.saturating_add(extra).min(MAX_CRANES_CAP)
    }
}
