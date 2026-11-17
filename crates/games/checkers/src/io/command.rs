use crate::model::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A whole move: start square, then every landing square of a capture.
    Move(Path),
}
