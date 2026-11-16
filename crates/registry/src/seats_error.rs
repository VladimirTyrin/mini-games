/// Seats of a match header that do not form the table `0..n`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SeatsError {
    #[error("seat slot {slot} is outside 0..{seat_count}")]
    SlotOutOfRange { slot: u32, seat_count: usize },
    #[error("seat slot {slot} appears twice")]
    DuplicateSlot { slot: u32 },
}
