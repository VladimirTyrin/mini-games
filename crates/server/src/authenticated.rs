use mg_protocol::proto;

use crate::auth::DisplayName;

/// Who a connection proved to be during Hello.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Authenticated {
    Guest { name: DisplayName },
    Google { subject: String, name: DisplayName },
    BotClient { name: DisplayName },
}

impl Authenticated {
    pub(crate) fn display_name(&self) -> &DisplayName {
        match self {
            Self::Guest { name } | Self::Google { name, .. } | Self::BotClient { name } => name,
        }
    }

    /// Accounts (Google, bot keys) are unique: a second sign-in takes the session over.
    /// Guests have no account, so two guests are never the same person.
    pub(crate) fn account_key(&self) -> Option<String> {
        match self {
            Self::Guest { .. } => None,
            Self::Google { subject, .. } => Some(format!("google:{subject}")),
            Self::BotClient { name } => Some(format!("bot:{}", name.uniqueness_key())),
        }
    }

    /// `display_name` is the name shown online, which may differ from the account's own name
    /// (see `Hub::online_name`).
    pub(crate) fn to_identity(
        &self,
        guest_id: &str,
        display_name: &DisplayName,
    ) -> proto::Identity {
        let (kind, id) = match self {
            Self::Guest { .. } => (proto::IdentityKind::Guest, guest_id.to_owned()),
            Self::Google { subject, .. } => (proto::IdentityKind::Google, subject.clone()),
            Self::BotClient { name } => (proto::IdentityKind::BotClient, name.as_str().to_owned()),
        };
        proto::Identity {
            kind: kind as i32,
            id,
            display_name: display_name.as_str().to_owned(),
        }
    }
}
