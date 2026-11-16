//! Fixed rule constants carried over from the legacy server.

/// Tick length; the UI converts tick counts (e.g. survival time) with it.
pub const TICK_MS: u32 = 200;

/// Workers occupy their feet cell and the cell above it.
pub const WORKER_HEIGHT: i32 = 2;

pub const POINTS_PER_LINE: u32 = 100;
/// Added once per extra line cleared in the same tick.
pub const MULTI_LINE_BONUS: u32 = 50;

pub const TICKS_PER_LEVEL: u64 = 150;
pub const INITIAL_CRANE_SPAWN_INTERVAL_TICKS: u32 = 15;
/// The spawn interval shrinks by this much per level, down to the minimum.
pub const CRANE_SPAWN_INTERVAL_STEP: u32 = 2;
pub const MIN_CRANE_SPAWN_INTERVAL_TICKS: u32 = 4;
pub const INITIAL_MAX_CRANES: u32 = 1;
/// One more simultaneous crane every this many levels, up to the cap.
pub const LEVELS_PER_EXTRA_CRANE: u32 = 2;
pub const MAX_CRANES_CAP: u32 = 5;

/// Points for clearing `lines` rows in one tick.
pub fn line_clear_points(lines: u32) -> u32 {
    let base = lines.saturating_mul(POINTS_PER_LINE);
    let bonus = lines.saturating_sub(1).saturating_mul(MULTI_LINE_BONUS);
    base.saturating_add(bonus)
}
