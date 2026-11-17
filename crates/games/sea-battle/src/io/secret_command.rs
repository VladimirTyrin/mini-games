use crate::model::Ship;

/// What a seat sends to the authority alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretCommand {
    /// The seat's fleet; the game checks it is a legal one (`Fleet::parse`).
    PlaceFleet(Vec<Ship>),
}
