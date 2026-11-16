use mg_protocol::proto;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FriendLinkKind {
    SingleUse,
    MultiUse,
}

impl FriendLinkKind {
    pub(crate) fn from_wire(kind: i32) -> Option<Self> {
        match proto::FriendLinkKind::try_from(kind).ok()? {
            proto::FriendLinkKind::SingleUse => Some(Self::SingleUse),
            proto::FriendLinkKind::MultiUse => Some(Self::MultiUse),
            proto::FriendLinkKind::Unspecified => None,
        }
    }

    pub(crate) fn to_wire(self) -> proto::FriendLinkKind {
        match self {
            Self::SingleUse => proto::FriendLinkKind::SingleUse,
            Self::MultiUse => proto::FriendLinkKind::MultiUse,
        }
    }

    pub(super) fn column(self) -> &'static str {
        match self {
            Self::SingleUse => "single",
            Self::MultiUse => "multi",
        }
    }

    pub(super) fn from_column(column: &str) -> Self {
        if column == "single" {
            Self::SingleUse
        } else {
            Self::MultiUse
        }
    }
}
