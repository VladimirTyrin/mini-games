use crate::model::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    Place(Point),
}
