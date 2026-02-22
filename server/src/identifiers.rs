use std::fmt;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(id: String) -> Self {
                Self(id)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(ClientId);

define_id!(LobbyId);

define_id!(SessionId);

define_id!(PlayerId);
define_id!(BotId);

impl BotId {
    pub fn to_player_id(&self) -> PlayerId {
        PlayerId::new(self.0.clone())
    }
}
