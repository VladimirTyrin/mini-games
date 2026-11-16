-- Google accounts the server has seen, so friends show a name and picture while offline.
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    avatar_url TEXT NOT NULL,
    last_seen_ms INTEGER NOT NULL
) STRICT;

-- Where an account's devices receive push notifications. `address` is the FCM token or the
-- Web Push endpoint; a device that signs in to another account moves its row there.
CREATE TABLE push_targets (
    address TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('fcm', 'web')),
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    updated_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX push_targets_by_account ON push_targets (account_id);

-- One row per friendship, the smaller id first.
CREATE TABLE friendships (
    low_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    high_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    since_ms INTEGER NOT NULL,
    PRIMARY KEY (low_id, high_id),
    CHECK (low_id < high_id)
) STRICT;
CREATE INDEX friendships_by_high ON friendships (high_id);

CREATE TABLE friend_requests (
    from_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    to_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    sent_ms INTEGER NOT NULL,
    PRIMARY KEY (from_id, to_id)
) STRICT;
CREATE INDEX friend_requests_by_recipient ON friend_requests (to_id);

CREATE TABLE friend_links (
    code TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('single', 'multi')),
    expires_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX friend_links_by_owner ON friend_links (owner_id);
