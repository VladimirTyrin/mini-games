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

    /// Accounts (Google, bot keys) are unique: a second sign-in takes the session over.
    /// Guests have no account, so two guests are never the same person.
    pub(crate) fn account(&self) -> Option<&PlayerId> {
        match self {
            Self::Guest { .. } => None,
            Self::Google { account, .. } | Self::BotClient { account, .. } => Some(account),
        }
    }

    /// A guest is a new player in every session; an account keeps its id.
    pub(crate) fn player_id(&self, session: &SessionId) -> PlayerId {
        self.account()
            .cloned()
            .unwrap_or_else(|| PlayerId::guest(session))
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
