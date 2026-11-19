use mg_core::EnglishText;

use crate::proto;

/// A game's [`mg_core::Game::Refusal`] on the wire: its case of [`proto::Refusal`]. Each game
/// maps its enum with an exhaustive `match`, so a new variant forces a choice: a wire value of
/// its own, or the generic `COMMON_REFUSAL_MALFORMED` for input no honest client sends.
pub trait WireRefusal: EnglishText {
    fn refusal_to_wire(self) -> proto::refusal::Reason;

    /// The variant behind one of the game's own wire values; `None` for `UNSPECIFIED` and values
    /// this build does not know.
    fn refusal_from_wire(value: i32) -> Option<Self>;
}
