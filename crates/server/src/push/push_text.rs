use std::borrow::Cow;

use mg_registry::GameKind;

/// A line of a push notification. The device words it in its language from the key and the
/// arguments (`client/push-texts.json` holds every key's texts). The English goes along in Web
/// Push, for a service worker from before the keys; FCM gets the keys alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PushText {
    AppTitle,
    InviteTitle { from: String },
    InviteBody { game: GameKind },
    InviteAcceptedElsewhere { from: String },
    InviteDeclinedElsewhere { from: String },
    FriendRequest { from: String },
    NewFriend { friend: String },
    Test,
}

impl PushText {
    /// The text's key: an Android string resource name, and the service worker's key.
    pub(crate) fn key(&self) -> Cow<'static, str> {
        match self {
            Self::AppTitle => Cow::Borrowed("push_title"),
            Self::InviteTitle { .. } => Cow::Borrowed("push_invite_title"),
            // A game's name cannot be an argument: Android puts arguments in as they are.
            Self::InviteBody { game } => Cow::Owned(format!("push_invite_body_{}", game.info().id)),
            Self::InviteAcceptedElsewhere { .. } => Cow::Borrowed("push_invite_accepted_elsewhere"),
            Self::InviteDeclinedElsewhere { .. } => Cow::Borrowed("push_invite_declined_elsewhere"),
            Self::FriendRequest { .. } => Cow::Borrowed("push_friend_request"),
            Self::NewFriend { .. } => Cow::Borrowed("push_new_friend"),
            Self::Test => Cow::Borrowed("push_test"),
        }
    }

    /// What `%1$s` stands for in the text.
    pub(crate) fn args(&self) -> Vec<&str> {
        match self {
            Self::InviteTitle { from }
            | Self::InviteAcceptedElsewhere { from }
            | Self::InviteDeclinedElsewhere { from }
            | Self::FriendRequest { from } => vec![from.as_str()],
            Self::NewFriend { friend } => vec![friend.as_str()],
            Self::AppTitle | Self::InviteBody { .. } | Self::Test => Vec::new(),
        }
    }

    pub(crate) fn english(&self) -> String {
        match self {
            Self::AppTitle => "Mini Games".to_owned(),
            Self::InviteTitle { from } => format!("{from} invites you to play"),
            Self::InviteBody { game } => format!("{}: tap to join.", game.info().name),
            Self::InviteAcceptedElsewhere { from } => {
                format!("You joined {from}'s game on another device.")
            }
            Self::InviteDeclinedElsewhere { from } => {
                format!("You declined {from}'s invitation on another device.")
            }
            Self::FriendRequest { from } => format!("{from} wants to be your friend."),
            Self::NewFriend { friend } => format!("{friend} is now your friend."),
            Self::Test => "Test notification: tap to see how long it took.".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every text the server sends, with "Sam" for each name.
    fn every_text() -> Vec<PushText> {
        let name = || "Sam".to_owned();
        let mut texts = vec![
            PushText::AppTitle,
            PushText::InviteTitle { from: name() },
            PushText::InviteAcceptedElsewhere { from: name() },
            PushText::InviteDeclinedElsewhere { from: name() },
            PushText::FriendRequest { from: name() },
            PushText::NewFriend { friend: name() },
            PushText::Test,
        ];
        texts.extend(
            GameKind::ALL
                .iter()
                .map(|&game| PushText::InviteBody { game }),
        );
        texts
    }

    #[test]
    fn every_key_has_its_words_on_the_devices_and_the_same_english() {
        let all: serde_json::Value =
            serde_json::from_str(include_str!("../../../../client/push-texts.json"))
                .expect("push-texts.json is JSON");
        for text in every_text() {
            let key = text.key();
            let english = all[key.as_ref()]["en"]
                .as_str()
                .unwrap_or_else(|| panic!("{key} is missing from push-texts.json"));
            let worded = text
                .args()
                .iter()
                .enumerate()
                .fold(english.to_owned(), |line, (at, arg)| {
                    line.replace(&format!("%{}$s", at + 1), arg)
                });
            // An invitation names the game as the app does ("Tic-Tac-Toe"), the English fallback
            // as the core does ("TicTacToe").
            if !matches!(text, PushText::InviteBody { .. }) {
                assert_eq!(worded, text.english(), "{key}");
            }
            assert!(all[key.as_ref()]["ru"].is_string(), "{key} has no Russian");
        }
    }
}
