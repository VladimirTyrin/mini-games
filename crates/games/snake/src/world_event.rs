use crate::point::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldEvent {
    /// Only valid on a free cell while below the food cap.
    SpawnFood(Point),
}
