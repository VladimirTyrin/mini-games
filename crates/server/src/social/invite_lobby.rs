use mg_registry::GameKind;

use crate::ids::LobbyId;

/// The lobby an invitation asks into, as the hub saw it when the invitation was made.
#[derive(Clone, Debug)]
pub(crate) struct InviteLobby {
    pub(crate) id: LobbyId,
    pub(crate) name: String,
    pub(crate) game: GameKind,
}
