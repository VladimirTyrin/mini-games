use mg_core::PlayerSpec;
use mg_protocol::proto;

use crate::seat_spec::SeatSpec;
use crate::seats_error::SeatsError;

/// Seats in slot order. The header's slots must be exactly `0..n`, each once; a seat is a bot
/// iff it names a `bot_kind`.
pub fn seats_from_header(header: &proto::MatchHeader) -> Result<Vec<SeatSpec>, SeatsError> {
    let seat_count = header.seats.len();
    let mut seats: Vec<Option<SeatSpec>> = vec![None; seat_count];
    for seat in &header.seats {
        let entry = usize::try_from(seat.slot)
            .ok()
            .and_then(|index| seats.get_mut(index))
            .ok_or(SeatsError::SlotOutOfRange {
                slot: seat.slot,
                seat_count,
            })?;
        if entry.is_some() {
            return Err(SeatsError::DuplicateSlot { slot: seat.slot });
        }
        *entry = Some(match seat.bot_kind {
            Some(kind) => SeatSpec::Bot(kind),
            None => SeatSpec::Human,
        });
    }
    // n distinct slots below n fill every entry.
    Ok(seats.into_iter().flatten().collect())
}

pub fn players_from_header(header: &proto::MatchHeader) -> Result<Vec<PlayerSpec>, SeatsError> {
    Ok(seats_from_header(header)?
        .iter()
        .map(SeatSpec::player_spec)
        .collect())
}
