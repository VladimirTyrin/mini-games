use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use mg_protocol::proto;
use rusqlite::{Connection, OptionalExtension, params};

use super::migrations::migrate;
use super::{
    AccountRecord, FriendGraph, FriendLinkKind, FriendLinkRecord, FriendLinkRefusal,
    FriendRequestDecision, FriendRequestOutcome, StoreError, UsedFriendLink,
};
use crate::ids::FriendLinkCode;
use crate::push::{FcmToken, PushTarget, WebPushSubscription};
use crate::social::AccountId;

/// The server's database: friends, friend links and push targets. Blocking; the social actor
/// calls it from blocking tasks. One connection behind a mutex is plenty for this traffic.
pub(crate) struct Store {
    connection: Mutex<Connection>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Store").finish_non_exhaustive()
    }
}

/// Times are unix milliseconds, passed in so tests control them.
impl Store {
    /// Push targets kept per account; the least recently registered go first.
    const MAX_PUSH_TARGETS: i64 = 10;

    /// Opens (creating it if needed) and migrates the database at `path`.
    pub(crate) fn open(path: &Path) -> Result<Self, StoreError> {
        let connection = Connection::open(path).map_err(StoreError::Sql)?;
        // WAL keeps readers off the writer's back; NORMAL sync is durable across process crashes
        // (a power loss may lose the last moments, which is fine for friend lists).
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .and_then(|()| connection.pragma_update(None, "synchronous", "NORMAL"))
            .map_err(StoreError::Sql)?;
        Self::prepare(connection)
    }

    /// A database that lives as long as the server (tests, or a server without MG_DATABASE).
    pub(crate) fn in_memory() -> Result<Self, StoreError> {
        Self::prepare(Connection::open_in_memory().map_err(StoreError::Sql)?)
    }

    fn prepare(mut connection: Connection) -> Result<Self, StoreError> {
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(StoreError::Sql)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(StoreError::Sql)?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn with<T>(
        &self,
        work: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> Result<T, StoreError> {
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        work(&mut connection).map_err(StoreError::Sql)
    }

    /// Records the name and picture an account signed in with.
    pub(crate) fn remember_account(
        &self,
        account: &AccountRecord,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        self.with(|connection| {
            connection.execute(
                "INSERT INTO accounts (id, display_name, avatar_url, last_seen_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (id) DO UPDATE SET
                     display_name = excluded.display_name,
                     avatar_url = excluded.avatar_url,
                     last_seen_ms = excluded.last_seen_ms",
                params![
                    account.id.as_str(),
                    account.display_name,
                    account.avatar_url,
                    now_ms
                ],
            )?;
            Ok(())
        })
    }

    pub(crate) fn account(&self, id: &AccountId) -> Result<Option<AccountRecord>, StoreError> {
        self.with(|connection| {
            connection
                .query_row(
                    "SELECT
                         display_name,
                         avatar_url
                     FROM accounts
                     WHERE id = ?1",
                    [id.as_str()],
                    |row| {
                        Ok(AccountRecord {
                            id: id.clone(),
                            display_name: row.get(0)?,
                            avatar_url: row.get(1)?,
                        })
                    },
                )
                .optional()
        })
    }

    pub(crate) fn save_push_target(
        &self,
        account: &AccountId,
        target: &PushTarget,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let (kind, p256dh, auth) = match target {
            PushTarget::Fcm(_) => ("fcm", String::new(), String::new()),
            PushTarget::WebPush(subscription) => {
                let wire = subscription.to_wire();
                ("web", wire.p256dh, wire.auth)
            }
        };
        self.with(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO push_targets (address, account_id, kind, p256dh, auth, updated_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (address) DO UPDATE SET
                     account_id = excluded.account_id,
                     kind = excluded.kind,
                     p256dh = excluded.p256dh,
                     auth = excluded.auth,
                     updated_ms = excluded.updated_ms",
                params![
                    target.address(),
                    account.as_str(),
                    kind,
                    p256dh,
                    auth,
                    now_ms
                ],
            )?;
            transaction.execute(
                "DELETE FROM push_targets
                 WHERE account_id = ?1
                   AND address NOT IN (
                       SELECT address
                       FROM push_targets
                       WHERE account_id = ?1
                       ORDER BY updated_ms DESC
                       LIMIT ?2
                   )",
                params![account.as_str(), Self::MAX_PUSH_TARGETS],
            )?;
            transaction.commit()
        })
    }

    pub(crate) fn push_targets(&self, account: &AccountId) -> Result<Vec<PushTarget>, StoreError> {
        let rows = self.with(|connection| {
            let mut statement = connection.prepare_cached(
                "SELECT
                     address,
                     kind,
                     p256dh,
                     auth
                 FROM push_targets
                 WHERE account_id = ?1",
            )?;
            statement
                .query_map([account.as_str()], |row| {
                    Ok(StoredPushTarget {
                        address: row.get(0)?,
                        kind: row.get(1)?,
                        p256dh: row.get(2)?,
                        auth: row.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
        })?;
        // Rows were checked when saved; one that no longer parses (rules got stricter) is skipped.
        Ok(rows
            .into_iter()
            .filter_map(StoredPushTarget::parse)
            .collect())
    }

    pub(crate) fn forget_push_target(&self, address: &str) -> Result<(), StoreError> {
        self.with(|connection| {
            connection.execute("DELETE FROM push_targets WHERE address = ?1", [address])?;
            Ok(())
        })
    }

    pub(crate) fn are_friends(&self, a: &AccountId, b: &AccountId) -> Result<bool, StoreError> {
        let (low, high) = ordered(a, b);
        self.with(|connection| {
            connection
                .query_row(
                    "SELECT 1 FROM friendships WHERE low_id = ?1 AND high_id = ?2",
                    [low.as_str(), high.as_str()],
                    |_| Ok(()),
                )
                .optional()
                .map(|found| found.is_some())
        })
    }

    pub(crate) fn friend_ids(&self, account: &AccountId) -> Result<Vec<AccountId>, StoreError> {
        Ok(self
            .friend_graph(account, i64::MAX)?
            .friends
            .into_iter()
            .map(|friend| friend.id)
            .collect())
    }

    /// Friends, open requests both ways and the account's links still valid at `now_ms`.
    pub(crate) fn friend_graph(
        &self,
        account: &AccountId,
        now_ms: i64,
    ) -> Result<FriendGraph, StoreError> {
        self.with(|connection| {
            let id = account.as_str();
            let friends = accounts_where(
                connection,
                "SELECT
                     a.id,
                     a.display_name,
                     a.avatar_url
                 FROM friendships f
                 JOIN accounts a ON a.id = CASE WHEN f.low_id = ?1 THEN f.high_id ELSE f.low_id END
                 WHERE f.low_id = ?1 OR f.high_id = ?1
                 ORDER BY a.display_name, a.id",
                id,
            )?;
            let incoming_requests = accounts_where(
                connection,
                "SELECT
                     a.id,
                     a.display_name,
                     a.avatar_url
                 FROM friend_requests r
                 JOIN accounts a ON a.id = r.from_id
                 WHERE r.to_id = ?1
                 ORDER BY r.sent_ms",
                id,
            )?;
            let outgoing_requests = accounts_where(
                connection,
                "SELECT
                     a.id,
                     a.display_name,
                     a.avatar_url
                 FROM friend_requests r
                 JOIN accounts a ON a.id = r.to_id
                 WHERE r.from_id = ?1
                 ORDER BY r.sent_ms",
                id,
            )?;
            let mut statement = connection.prepare_cached(
                "SELECT
                     code,
                     kind,
                     expires_ms
                 FROM friend_links
                 WHERE owner_id = ?1 AND expires_ms > ?2
                 ORDER BY expires_ms",
            )?;
            let links = statement
                .query_map(params![id, now_ms], |row| {
                    let code: String = row.get(0)?;
                    let kind: String = row.get(1)?;
                    Ok(FriendLinkCode::parse(&code).map(|code| FriendLinkRecord {
                        code,
                        kind: FriendLinkKind::from_column(&kind),
                        expires_ms: row.get(2).unwrap_or(0),
                    }))
                })?
                .filter_map(Result::transpose)
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(FriendGraph {
                friends,
                incoming_requests,
                outgoing_requests,
                links,
            })
        })
    }

    pub(crate) fn send_friend_request(
        &self,
        from: &AccountId,
        to: &AccountId,
        now_ms: i64,
    ) -> Result<FriendRequestOutcome, StoreError> {
        let (low, high) = ordered(from, to);
        self.with(|connection| {
            let transaction = connection.transaction()?;
            let friends = transaction
                .query_row(
                    "SELECT 1 FROM friendships WHERE low_id = ?1 AND high_id = ?2",
                    [low.as_str(), high.as_str()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if friends {
                return Ok(FriendRequestOutcome::AlreadyFriends);
            }
            let answered = transaction.execute(
                "DELETE FROM friend_requests WHERE from_id = ?1 AND to_id = ?2",
                [to.as_str(), from.as_str()],
            )?;
            let outcome = if answered > 0 {
                befriend(&transaction, from, to, now_ms)?;
                FriendRequestOutcome::BecameFriends
            } else {
                let inserted = transaction.execute(
                    "INSERT INTO friend_requests (from_id, to_id, sent_ms)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT DO NOTHING",
                    params![from.as_str(), to.as_str(), now_ms],
                )?;
                if inserted > 0 {
                    FriendRequestOutcome::Sent
                } else {
                    FriendRequestOutcome::AlreadySent
                }
            };
            transaction.commit()?;
            Ok(outcome)
        })
    }

    /// Accepts or declines the request `from` sent to `to`; `false` when there is none.
    pub(crate) fn answer_friend_request(
        &self,
        to: &AccountId,
        from: &AccountId,
        accept: FriendRequestDecision,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        self.with(|connection| {
            let transaction = connection.transaction()?;
            let found = transaction.execute(
                "DELETE FROM friend_requests WHERE from_id = ?1 AND to_id = ?2",
                [from.as_str(), to.as_str()],
            )? > 0;
            if found && accept == FriendRequestDecision::Accept {
                befriend(&transaction, from, to, now_ms)?;
            }
            transaction.commit()?;
            Ok(found)
        })
    }

    /// Ends the friendship and drops requests either way.
    pub(crate) fn remove_friend(&self, a: &AccountId, b: &AccountId) -> Result<(), StoreError> {
        let (low, high) = ordered(a, b);
        self.with(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "DELETE FROM friendships WHERE low_id = ?1 AND high_id = ?2",
                [low.as_str(), high.as_str()],
            )?;
            transaction.execute(
                "DELETE FROM friend_requests
                 WHERE (from_id = ?1 AND to_id = ?2) OR (from_id = ?2 AND to_id = ?1)",
                [a.as_str(), b.as_str()],
            )?;
            transaction.commit()
        })
    }

    pub(crate) fn active_link_count(
        &self,
        owner: &AccountId,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        let count: i64 = self.with(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM friend_links WHERE owner_id = ?1 AND expires_ms > ?2",
                params![owner.as_str(), now_ms],
                |row| row.get(0),
            )
        })?;
        Ok(usize::try_from(count).unwrap_or(0))
    }

    pub(crate) fn create_friend_link(
        &self,
        owner: &AccountId,
        link: &FriendLinkRecord,
    ) -> Result<(), StoreError> {
        self.with(|connection| {
            connection.execute(
                "INSERT INTO friend_links (code, owner_id, kind, expires_ms)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    link.code.as_str(),
                    owner.as_str(),
                    link.kind.column(),
                    link.expires_ms
                ],
            )?;
            Ok(())
        })
    }

    pub(crate) fn revoke_friend_link(
        &self,
        owner: &AccountId,
        code: &FriendLinkCode,
    ) -> Result<(), StoreError> {
        self.with(|connection| {
            connection.execute(
                "DELETE FROM friend_links WHERE code = ?1 AND owner_id = ?2",
                [code.as_str(), owner.as_str()],
            )?;
            Ok(())
        })
    }

    /// Makes `user` a friend of the link's owner; a single-use link is gone afterwards.
    pub(crate) fn use_friend_link(
        &self,
        code: &FriendLinkCode,
        user: &AccountId,
        now_ms: i64,
    ) -> Result<Result<UsedFriendLink, FriendLinkRefusal>, StoreError> {
        self.with(|connection| {
            let transaction = connection.transaction()?;
            let link = transaction
                .query_row(
                    "SELECT
                         owner_id,
                         kind
                     FROM friend_links
                     WHERE code = ?1 AND expires_ms > ?2",
                    params![code.as_str(), now_ms],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            let Some((owner, kind)) = link else {
                return Ok(Err(FriendLinkRefusal::Invalid));
            };
            let Ok(owner) = AccountId::parse(&owner) else {
                return Ok(Err(FriendLinkRefusal::Invalid));
            };
            if &owner == user {
                return Ok(Err(FriendLinkRefusal::OwnLink));
            }
            let (low, high) = ordered(&owner, user);
            let already_friends = transaction
                .query_row(
                    "SELECT 1 FROM friendships WHERE low_id = ?1 AND high_id = ?2",
                    [low.as_str(), high.as_str()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            // A friend opening the link again changes nothing: a single-use link stays for the
            // friend it was meant for.
            if !already_friends {
                if FriendLinkKind::from_column(&kind) == FriendLinkKind::SingleUse {
                    transaction
                        .execute("DELETE FROM friend_links WHERE code = ?1", [code.as_str()])?;
                }
                befriend(&transaction, &owner, user, now_ms)?;
            }
            transaction.commit()?;
            Ok(Ok(UsedFriendLink {
                owner,
                became_friends: !already_friends,
            }))
        })
    }

    pub(crate) fn delete_expired_links(&self, now_ms: i64) -> Result<(), StoreError> {
        self.with(|connection| {
            connection.execute("DELETE FROM friend_links WHERE expires_ms <= ?1", [now_ms])?;
            Ok(())
        })
    }
}

/// Also clears open requests between the two, which the friendship answers.
fn befriend(
    transaction: &rusqlite::Transaction<'_>,
    a: &AccountId,
    b: &AccountId,
    now_ms: i64,
) -> rusqlite::Result<()> {
    let (low, high) = ordered(a, b);
    transaction.execute(
        "INSERT INTO friendships (low_id, high_id, since_ms)
         VALUES (?1, ?2, ?3)
         ON CONFLICT DO NOTHING",
        params![low.as_str(), high.as_str(), now_ms],
    )?;
    transaction.execute(
        "DELETE FROM friend_requests
         WHERE (from_id = ?1 AND to_id = ?2) OR (from_id = ?2 AND to_id = ?1)",
        [a.as_str(), b.as_str()],
    )?;
    Ok(())
}

fn ordered<'a>(a: &'a AccountId, b: &'a AccountId) -> (&'a AccountId, &'a AccountId) {
    if a <= b { (a, b) } else { (b, a) }
}

fn accounts_where(
    connection: &Connection,
    query: &str,
    id: &str,
) -> rusqlite::Result<Vec<AccountRecord>> {
    let mut statement = connection.prepare_cached(query)?;
    statement
        .query_map([id], |row| {
            let id: String = row.get(0)?;
            Ok(AccountId::parse(&id).ok().map(|id| AccountRecord {
                id,
                display_name: row.get(1).unwrap_or_default(),
                avatar_url: row.get(2).unwrap_or_default(),
            }))
        })?
        .filter_map(Result::transpose)
        .collect()
}

struct StoredPushTarget {
    address: String,
    kind: String,
    p256dh: String,
    auth: String,
}

impl StoredPushTarget {
    fn parse(self) -> Option<PushTarget> {
        match self.kind.as_str() {
            "fcm" => FcmToken::parse(self.address).ok().map(PushTarget::Fcm),
            _ => WebPushSubscription::parse(proto::WebPushSubscription {
                endpoint: self.address,
                p256dh: self.p256dh,
                auth: self.auth,
            })
            .ok()
            .map(PushTarget::WebPush),
        }
    }
}
