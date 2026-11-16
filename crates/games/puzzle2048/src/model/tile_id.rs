/// Stable tile identity: a tile keeps its id while it slides; spawns and merges get fresh ids.
/// Ids are handed out in increasing order from 0 and never reused within a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileId(u32);

impl TileId {
    pub(crate) const FIRST: TileId = TileId(0);

    pub fn get(self) -> u32 {
        self.0
    }

    /// Wrapping is unreachable in play: every id comes from a spawn or a merge, and the number of
    /// spawns is bounded because each one grows the (target-bounded) sum of tile values.
    pub(crate) fn next(self) -> TileId {
        TileId(self.0.wrapping_add(1))
    }
}
