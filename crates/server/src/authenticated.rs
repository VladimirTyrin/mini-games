use mg_protocol::proto;

use crate::auth::{AvatarUrl, DisplayName};
use crate::ids::SessionId;
use crate::player_id::PlayerId;

/// Who a connection proved to be during Hello.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Authenticated {
    Guest {
        name: DisplayName,
    },
    Google {
        account: PlayerId,
        name: DisplayName,
        avatar: Option<AvatarUrl>,
    },
    BotClient {
        account: PlayerId,
        name: DisplayName,
    },
}

impl Authenticated {
    pub(crate) fn display_name(&self) -> &DisplayName {
        match self {
            Self::Guest { name } | Self::Google { name, .. } | Self::BotClient { name, .. } => name,
        }
    }

    /// A bot key is one player: a second sign-in with it takes the session over (a restarted
    /// bot process gets its seat back). A Google account plays from several devices at once,
    /// each device its own player, so it never takes a session over; guests have no account.
    pub(crate) fn exclusive_account(&self) -> Option<&PlayerId> {
        match self {
            Self::BotClient { account, .. } => Some(account),
            Self::Guest { .. } | Self::Google { .. } => None,
        }
    }

    /// Whether a resumed session belongs to this sign-in. An account matches by account, so a
    /// changed profile name or picture still resumes; a guest by name.
    pub(crate) fn is_same_player(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Google { account: a, .. }, Self::Google { account: b, .. })
            | (Self::BotClient { account: a, .. }, Self::BotClient { account: b, .. }) => a == b,
            (Self::Guest { name: a }, Self::Guest { name: b }) => a == b,
            _ => false,
        }
    }

    /// A guest is a new player in every session, and so is each device of a Google account; a
    /// bot keeps its id.
    pub(crate) fn player_id(&self, session: &SessionId) -> PlayerId {
        match self {
            Self::Guest { .. } => PlayerId::guest(session),
            Self::Google { account, .. } => PlayerId::device_of(account, session),
            Self::BotClient { account, .. } => account.clone(),
        }
    }

    pub(crate) fn to_identity(&self, session: &SessionId) -> proto::Identity {
        let kind = match self {
            Self::Guest { .. } => proto::IdentityKind::Guest,
            Self::Google { .. } => proto::IdentityKind::Google,
            Self::BotClient { .. } => proto::IdentityKind::BotClient,
        };
        let avatar_url = match self {
            Self::Google {
                avatar: Some(avatar),
                ..
            } => avatar.as_str().to_owned(),
            Self::Guest { .. } | Self::Google { .. } | Self::BotClient { .. } => String::new(),
        };
        proto::Identity {
            kind: kind as i32,
            id: self.player_id(session).as_str().to_owned(),
            display_name: self.display_name().as_str().to_owned(),
            avatar_url,
        }
    }
}
