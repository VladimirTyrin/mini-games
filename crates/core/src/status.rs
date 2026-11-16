use crate::outcome::Outcome;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    InProgress,
    Finished(Outcome),
}

impl Status {
    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Finished(_))
    }
}
