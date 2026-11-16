use mg_core::Slot;

use crate::ids::SessionId;
use crate::outbound::Outbound;

#[derive(Debug)]
pub(crate) struct Participant {
    pub session: SessionId,
    /// `None` for observers.
    pub slot: Option<Slot>,
    /// `None` while disconnected.
    pub outbound: Option<Outbound>,
}
