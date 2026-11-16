use crate::column::Column;
use crate::pattern_id::PatternId;
use crate::side::Side;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldEvent {
    /// Valid only when a spawn is due (interval elapsed and below the level's crane cap) and
    /// `target_x` is a field column.
    CraneSpawned {
        target_x: Column,
        from: Side,
        pattern: PatternId,
    },
}
