use std::fmt;

/// Random hex id of `bytes * 2` characters.
fn random_hex(bytes: usize) -> String {
    let mut out = String::with_capacity(bytes * 2);
    for _ in 0..bytes {
        let byte: u8 = rand::random();
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

macro_rules! string_id {
    ($name:ident, $bytes:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub(crate) struct $name(std::sync::Arc<str>);

        // Not every id type uses every helper.
        #[allow(dead_code)]
        impl $name {
            pub(crate) fn random() -> Self {
                Self(random_hex($bytes).into())
            }

            pub(crate) fn parse(raw: &str) -> Option<Self> {
                let valid = raw.len() == $bytes * 2 && raw.bytes().all(|b| b.is_ascii_hexdigit());
                valid.then(|| Self(raw.to_ascii_lowercase().into()))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(SessionId, 8);
string_id!(LobbyId, 4);
string_id!(MatchId, 8);
string_id!(InviteId, 8);
// Whoever has it can become the owner's friend while it lasts: 80 bits, not guessable.
string_id!(FriendLinkCode, 10);
// Secret: whoever holds it can take over the session.
string_id!(ResumeToken, 16);

/// Identifies one WebSocket connection; sessions outlive connections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ConnectionId(u64);

impl ConnectionId {
    pub(crate) fn next() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "conn-{}", self.0)
    }
}
